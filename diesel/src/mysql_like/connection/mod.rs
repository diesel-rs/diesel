mod bind;
mod raw;
mod stmt;
mod url;

use core::num::NonZeroU64;

use self::raw::RawConnection;
use self::stmt::Statement;
use self::stmt::iterator::StatementIterator;
use self::url::ConnectionOptions;
use crate::RunQueryDsl;
use crate::connection::instrumentation::{DebugQuery, DynInstrumentation, StrQueryHelper};
use crate::connection::statement_cache::{MaybeCached, StatementCache};
use crate::connection::*;
use crate::expression::QueryMetadata;
use crate::mysql_like::MysqlLikeBackend;
use crate::query_builder::bind_collector::RawBytesBindCollector;
use crate::query_builder::*;
use crate::result::*;

#[allow(missing_debug_implementations, missing_copy_implementations)]
/// A connection to a MySQL database. Connection URLs should be in the form
/// `mysql://[user[:password]@]host/database_name[?unix_socket=socket-path&ssl_mode=SSL_MODE*&ssl_ca=/etc/ssl/certs/ca-certificates.crt&ssl_cert=/etc/ssl/certs/client-cert.crt&ssl_key=/etc/ssl/certs/client-key.crt]`
///
///* `host` can be an IP address or a hostname. If it is set to `localhost`, a connection
///  will be attempted through the socket at `/tmp/mysql.sock`. If you want to connect to
///  a local server via TCP (e.g. docker containers), use `0.0.0.0` or `127.0.0.1` instead.
/// * `unix_socket` expects the path to the unix socket
/// * `ssl_ca` accepts a path to the system's certificate roots
/// * `ssl_cert` accepts a path to the client's certificate file
/// * `ssl_key` accepts a path to the client's private key file
/// * `ssl_mode` expects a value defined for MySQL client command option `--ssl-mode`
///   See <https://dev.mysql.com/doc/refman/5.7/en/connection-options.html#option_general_ssl-mode>
///
/// # Supported loading model implementations
///
/// * [`DefaultLoadingMode`](crate::connection::DefaultLoadingMode)
///
/// As `MysqlConnection` only supports a single loading mode implementation
/// it is **not required** to explicitly specify a loading mode
/// when calling [`RunQueryDsl::load_iter()`] or [`LoadConnection::load`]
///
/// ## DefaultLoadingMode
///
/// `MysqlConnection` only supports a single loading mode, which loads
/// values row by row from the result set.
///
/// ```rust
/// # include!("../../doctest_setup.rs");
/// #
/// # fn main() {
/// #     run_test().unwrap();
/// # }
/// #
/// # fn run_test() -> QueryResult<()> {
/// #     use schema::users;
/// #     let connection = &mut establish_connection();
/// use diesel::connection::DefaultLoadingMode;
/// {
///     // scope to restrict the lifetime of the iterator
///     let iter1 = users::table.load_iter::<(i32, String), DefaultLoadingMode>(connection)?;
///
///     for r in iter1 {
///         let (id, name) = r?;
///         println!("Id: {} Name: {}", id, name);
///     }
/// }
///
/// // works without specifying the loading mode
/// let iter2 = users::table.load_iter::<(i32, String), _>(connection)?;
///
/// for r in iter2 {
///     let (id, name) = r?;
///     println!("Id: {} Name: {}", id, name);
/// }
/// #   Ok(())
/// # }
/// ```
///
/// This mode does **not support** creating
/// multiple iterators using the same connection.
///
/// ```compile_fail
/// # include!("../../doctest_setup.rs");
/// #
/// # fn main() {
/// #     run_test().unwrap();
/// # }
/// #
/// # fn run_test() -> QueryResult<()> {
/// #     use schema::users;
/// #     let connection = &mut establish_connection();
/// use diesel::connection::DefaultLoadingMode;
///
/// let iter1 = users::table.load_iter::<(i32, String), DefaultLoadingMode>(connection)?;
/// let iter2 = users::table.load_iter::<(i32, String), DefaultLoadingMode>(connection)?;
///
/// for r in iter1 {
///     let (id, name) = r?;
///     println!("Id: {} Name: {}", id, name);
/// }
///
/// for r in iter2 {
///     let (id, name) = r?;
///     println!("Id: {} Name: {}", id, name);
/// }
/// #   Ok(())
/// # }
/// ```
pub struct MysqlLikeConnection<DB: MysqlLikeBackend> {
    raw_connection: RawConnection,
    transaction_state: AnsiTransactionManager,
    statement_cache: StatementCache<DB, Statement<DB>>,
    instrumentation: DynInstrumentation,
}

// mysql connection can be shared between threads according to libmysqlclients documentation
#[allow(unsafe_code)]
unsafe impl<DB: MysqlLikeBackend> Send for MysqlLikeConnection<DB> {}

impl<DB: MysqlLikeBackend> SimpleConnection for MysqlLikeConnection<DB> {
    fn batch_execute(&mut self, query: &str) -> QueryResult<()> {
        self.instrumentation
            .on_connection_event(InstrumentationEvent::StartQuery {
                query: &StrQueryHelper::new(query),
            });
        let r = self
            .raw_connection
            .enable_multi_statements(|| self.raw_connection.execute(query));
        self.instrumentation
            .on_connection_event(InstrumentationEvent::FinishQuery {
                query: &StrQueryHelper::new(query),
                error: r.as_ref().err(),
            });
        r
    }
}

impl<DB: MysqlLikeBackend> ConnectionSealed for MysqlLikeConnection<DB> {}

impl<DB: MysqlLikeBackend> Connection for MysqlLikeConnection<DB> {
    type Backend = DB;
    type TransactionManager = AnsiTransactionManager;

    /// Establishes a new connection to the MySQL database
    /// `database_url` may be enhanced by GET parameters
    /// `mysql://[user[:password]@]host[:port]/database_name[?unix_socket=socket-path&ssl_mode=SSL_MODE*&ssl_ca=/etc/ssl/certs/ca-certificates.crt&ssl_cert=/etc/ssl/certs/client-cert.crt&ssl_key=/etc/ssl/certs/client-key.crt&local_infile=true]`
    ///
    /// * `host` can be an IP address or a hostname. If it is set to `localhost`, a connection
    ///   will be attempted through the socket at `/tmp/mysql.sock`. If you want to connect to
    ///   a local server via TCP (e.g. docker containers), use `0.0.0.0` or `127.0.0.1` instead.
    /// * `unix_socket` expects the path to the unix socket
    /// * `ssl_ca` accepts a path to the system's certificate roots
    /// * `ssl_cert` accepts a path to the client's certificate file
    /// * `ssl_key` accepts a path to the client's private key file
    /// * `ssl_mode` expects a value defined for MySQL client command option `--ssl-mode`
    ///   See <https://dev.mysql.com/doc/refman/5.7/en/connection-options.html#option_general_ssl-mode>
    /// * `local_infile` expects a boolean to enable or disable LOAD DATA LOCAL
    fn establish(database_url: &str) -> ConnectionResult<Self> {
        let mut instrumentation = DynInstrumentation::default_instrumentation();
        instrumentation.on_connection_event(InstrumentationEvent::StartEstablishConnection {
            url: database_url,
        });

        let establish_result = Self::establish_inner(database_url);
        instrumentation.on_connection_event(InstrumentationEvent::FinishEstablishConnection {
            url: database_url,
            error: establish_result.as_ref().err(),
        });
        let mut conn = establish_result?;
        conn.instrumentation = instrumentation;
        Ok(conn)
    }

    fn execute_returning_count<T>(&mut self, source: &T) -> QueryResult<usize>
    where
        T: QueryFragment<Self::Backend> + QueryId,
    {
        #[allow(unsafe_code)] // call to unsafe function
        update_transaction_manager_status(
            prepared_query(
                &source,
                &mut self.statement_cache,
                &mut self.raw_connection,
                &mut *self.instrumentation,
            )
            .and_then(|stmt| {
                // we have not called result yet, so calling `execute` is
                // fine
                let stmt_use = unsafe { stmt.execute() }?;
                stmt_use.affected_rows()
            }),
            &mut self.transaction_state,
            &mut self.instrumentation,
            &crate::debug_query(source),
        )
    }

    fn transaction_state(&mut self) -> &mut AnsiTransactionManager {
        &mut self.transaction_state
    }

    fn instrumentation(&mut self) -> &mut dyn Instrumentation {
        &mut *self.instrumentation
    }

    fn set_instrumentation(&mut self, instrumentation: impl Instrumentation) {
        self.instrumentation = instrumentation.into();
    }

    fn set_prepared_statement_cache_size(&mut self, size: CacheSize) {
        self.statement_cache.set_cache_size(size);
    }
}

#[inline(always)]
fn update_transaction_manager_status<T>(
    query_result: QueryResult<T>,
    transaction_manager: &mut AnsiTransactionManager,
    instrumentation: &mut DynInstrumentation,
    query: &dyn DebugQuery,
) -> QueryResult<T> {
    fn non_generic_inner(
        query_result: Result<(), &Error>,
        transaction_manager: &mut AnsiTransactionManager,
        instrumentation: &mut DynInstrumentation,
        query: &dyn DebugQuery,
    ) {
        if let Err(Error::DatabaseError(DatabaseErrorKind::SerializationFailure, _)) = query_result
        {
            transaction_manager
                .status
                .set_requires_rollback_maybe_up_to_top_level(true)
        }
        instrumentation.on_connection_event(InstrumentationEvent::FinishQuery {
            query,
            error: query_result.err(),
        });
    }

    non_generic_inner(
        query_result.as_ref().map(|_| ()),
        transaction_manager,
        instrumentation,
        query,
    );
    query_result
}

impl<DB: MysqlLikeBackend> LoadConnection<DefaultLoadingMode> for MysqlLikeConnection<DB> {
    type Cursor<'conn, 'query> = self::stmt::iterator::StatementIterator<'conn, DB>;
    type Row<'conn, 'query> = self::stmt::iterator::MysqlRow<DB>;

    fn load<'conn, 'query, T>(
        &'conn mut self,
        source: T,
    ) -> QueryResult<Self::Cursor<'conn, 'query>>
    where
        T: Query + QueryFragment<Self::Backend> + QueryId + 'query,
        Self::Backend: QueryMetadata<T::SqlType>,
    {
        update_transaction_manager_status(
            prepared_query(
                &source,
                &mut self.statement_cache,
                &mut self.raw_connection,
                &mut *self.instrumentation,
            )
            .and_then(|stmt| {
                let mut metadata = Vec::new();
                DB::row_metadata(&mut (), &mut metadata);
                StatementIterator::from_stmt(stmt, &metadata)
            }),
            &mut self.transaction_state,
            &mut self.instrumentation,
            &crate::debug_query(&source),
        )
    }
}

#[cfg(feature = "r2d2")]
impl<DB: MysqlLikeBackend> crate::r2d2::R2D2Connection for MysqlLikeConnection<DB> {
    fn ping(&mut self) -> QueryResult<()> {
        crate::r2d2::CheckConnectionQuery.execute(self).map(|_| ())
    }

    fn is_broken(&mut self) -> bool {
        AnsiTransactionManager::is_broken_transaction_manager(self)
    }
}

impl<DB: MysqlLikeBackend> MultiConnectionHelper for MysqlLikeConnection<DB> {
    fn to_any<'a>(
        lookup: &mut <Self::Backend as crate::sql_types::TypeMetadata>::MetadataLookup,
    ) -> &mut (dyn core::any::Any + 'a) {
        lookup
    }

    fn from_any(
        lookup: &mut dyn core::any::Any,
    ) -> Option<&mut <Self::Backend as crate::sql_types::TypeMetadata>::MetadataLookup> {
        lookup.downcast_mut()
    }
}

fn prepared_query<'a, DB: MysqlLikeBackend + Default, T: QueryFragment<DB> + QueryId>(
    source: &'_ T,
    statement_cache: &'a mut StatementCache<DB, Statement<DB>>,
    raw_connection: &'a mut RawConnection,
    instrumentation: &mut dyn Instrumentation,
) -> QueryResult<MaybeCached<'a, Statement<DB>>> {
    instrumentation.on_connection_event(InstrumentationEvent::StartQuery {
        query: &crate::debug_query(source),
    });
    let mut stmt = statement_cache.cached_statement(
        source,
        &DB::default(),
        &[],
        &*raw_connection,
        RawConnection::prepare,
        instrumentation,
    )?;

    let mut bind_collector = RawBytesBindCollector::new();
    source.collect_binds(&mut bind_collector, &mut (), &DB::default())?;
    let binds = bind_collector
        .metadata
        .into_iter()
        .zip(bind_collector.binds);
    stmt.bind(binds)?;
    Ok(stmt)
}

impl<DB: MysqlLikeBackend> MysqlLikeConnection<DB> {
    /// Executes `source` and returns its `mysql_stmt_insert_id`, zero mapped
    /// to `None`. Public entry point: [`InsertStatement::execute_returning_id`].
    pub(crate) fn execute_returning_id<T>(&mut self, source: &T) -> QueryResult<Option<NonZeroU64>>
    where
        T: QueryFragment<DB> + QueryId,
    {
        #[allow(unsafe_code)] // call to unsafe function
        update_transaction_manager_status(
            prepared_query(
                &source,
                &mut self.statement_cache,
                &mut self.raw_connection,
                &mut *self.instrumentation,
            )
            .and_then(|stmt| {
                // SAFETY: `prepared_query` returned this statement freshly
                // bound, so no result set is pending, which is `execute`'s
                // requirement.
                let stmt_use = unsafe { stmt.execute() }?;
                Ok(NonZeroU64::new(stmt_use.insert_id()))
            }),
            &mut self.transaction_state,
            &mut self.instrumentation,
            &crate::debug_query(source),
        )
    }

    fn set_config_options(&mut self) -> QueryResult<()> {
        crate::sql_query("SET time_zone = '+00:00';").execute(self)?;
        crate::sql_query("SET character_set_client = 'utf8mb4'").execute(self)?;
        crate::sql_query("SET character_set_connection = 'utf8mb4'").execute(self)?;
        crate::sql_query("SET character_set_results = 'utf8mb4'").execute(self)?;
        Ok(())
    }

    fn establish_inner(database_url: &str) -> Result<MysqlLikeConnection<DB>, ConnectionError> {
        use crate::ConnectionError::CouldntSetupConfiguration;

        let raw_connection = RawConnection::new();
        let connection_options = ConnectionOptions::parse::<DB>(database_url)?;
        raw_connection.connect(&connection_options)?;
        let mut conn = MysqlLikeConnection {
            raw_connection,
            transaction_state: AnsiTransactionManager::default(),
            statement_cache: StatementCache::new(),
            instrumentation: DynInstrumentation::none(),
        };
        conn.set_config_options()
            .map_err(CouldntSetupConfiguration)?;
        Ok(conn)
    }
}
