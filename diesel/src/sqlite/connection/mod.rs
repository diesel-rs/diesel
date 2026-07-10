#[cfg(not(all(target_family = "wasm", target_os = "unknown")))]
extern crate libsqlite3_sys as ffi;

#[cfg(all(target_family = "wasm", target_os = "unknown"))]
use sqlite_wasm_rs as ffi;

pub mod authorizer;
mod bind_collector;
mod functions;
mod hooks;
mod limits;
mod owned_row;
mod raw;
mod row;
mod serialized_database;
pub(in crate::sqlite) mod sqlite_blob;
mod sqlite_value;
mod statement_iterator;
mod stmt;
mod trace;

pub use self::authorizer::{AuthorizerContext, AuthorizerDecision};
pub(in crate::sqlite) use self::bind_collector::SqliteBindCollector;
pub use self::bind_collector::SqliteBindValue;
pub use self::limits::SqliteLimit;
pub use self::serialized_database::SerializedDatabase;
pub use self::sqlite_value::SqliteValue;
pub use self::trace::{SqliteTraceEvent, SqliteTraceFlags};

use self::raw::RawConnection;
use self::statement_iterator::*;
use self::stmt::{Statement, StatementUse};
use super::SqliteAggregateFunction;
use crate::connection::instrumentation::{DynInstrumentation, StrQueryHelper};
use crate::connection::statement_cache::StatementCache;
use crate::connection::*;
use crate::deserialize::{FromSqlRow, StaticallySizedRow};
use crate::expression::QueryMetadata;
use crate::query_builder::*;
use crate::result::*;
use crate::serialize::ToSql;
use crate::sql_types::{HasSqlType, TypeMetadata};
use crate::sqlite::{Sqlite, SqliteFunctionBehavior};
use alloc::string::String;
use alloc::vec::Vec;
use core::ffi as libc;
use core::num::NonZeroI64;

/// Connections for the SQLite backend. Unlike other backends, SQLite supported
/// connection URLs are:
///
/// - File paths (`test.db`)
/// - [URIs](https://sqlite.org/uri.html) (`file://test.db`)
/// - Special identifiers (`:memory:`)
///
/// # Supported loading model implementations
///
/// * [`DefaultLoadingMode`]
///
/// As `SqliteConnection` only supports a single loading mode implementation,
/// it is **not required** to explicitly specify a loading mode
/// when calling [`RunQueryDsl::load_iter()`] or [`LoadConnection::load`]
///
/// [`RunQueryDsl::load_iter()`]: crate::query_dsl::RunQueryDsl::load_iter
///
/// ## DefaultLoadingMode
///
/// `SqliteConnection` only supports a single loading mode, which loads
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
///
/// # Concurrency
///
/// By default, when running into a database lock, the operation will abort with a
/// `Database locked` error. However, it's possible to configure it for greater concurrency,
/// trading latency for not having to deal with retries yourself.
///
/// You can use this example as blue-print for which statements to run after establishing a connection.
/// It is **important** to run each `PRAGMA` in a single statement to make sure all of them apply
/// correctly. In addition the order of the `PRAGMA` statements is relevant to prevent timeout
/// issues for the later `PRAGMA` statements.
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
/// use diesel::connection::SimpleConnection;
/// let conn = &mut establish_connection();
/// // see https://fractaledmind.github.io/2023/09/07/enhancing-rails-sqlite-fine-tuning/
/// // sleep if the database is busy, this corresponds to up to 2 seconds sleeping time.
/// conn.batch_execute("PRAGMA busy_timeout = 2000;")?;
/// // better write-concurrency
/// conn.batch_execute("PRAGMA journal_mode = WAL;")?;
/// // fsync only in critical moments
/// conn.batch_execute("PRAGMA synchronous = NORMAL;")?;
/// // write WAL changes back every 1000 pages, for an in average 1MB WAL file.
/// // May affect readers if number is increased
/// conn.batch_execute("PRAGMA wal_autocheckpoint = 1000;")?;
/// // free some space by truncating possibly massive WAL files from the last run
/// conn.batch_execute("PRAGMA wal_checkpoint(TRUNCATE);")?;
/// #   Ok(())
/// # }
/// ```
#[allow(missing_debug_implementations)]
#[cfg(feature = "__sqlite-shared")]
pub struct SqliteConnection {
    // statement_cache needs to be before raw_connection
    // otherwise we will get errors about open statements before closing the
    // connection itself
    statement_cache: StatementCache<Sqlite, Statement>,
    raw_connection: RawConnection,
    transaction_state: AnsiTransactionManager,
    // this exists for the sole purpose of implementing `WithMetadataLookup` trait
    // and avoiding static mut which will be deprecated in 2024 edition
    metadata_lookup: (),
    instrumentation: DynInstrumentation,
    // We potentially need to store a serialized
    // database in here to make sure the database bytes
    // live as long as the connection
    // This is used by SqliteConnection::deserialize_readonly_database_from_buffer
    // only
    // This field needs to come after the RawConnection
    // as we need to make sure the data are still there until the
    // connection is dropped
    //
    // We are not allowed to modify the inner buffer until the database connection is dropped
    serialized_data: Vec<Vec<u8>>,
}

// This relies on the invariant that RawConnection or Statement are never
// leaked. If a reference to one of those was held on a different thread, this
// would not be thread safe.
#[allow(unsafe_code)]
unsafe impl Send for SqliteConnection {}

impl SimpleConnection for SqliteConnection {
    fn batch_execute(&mut self, query: &str) -> QueryResult<()> {
        self.instrumentation
            .on_connection_event(InstrumentationEvent::StartQuery {
                query: &StrQueryHelper::new(query),
            });
        let resp = self.raw_connection.exec(query);
        self.instrumentation
            .on_connection_event(InstrumentationEvent::FinishQuery {
                query: &StrQueryHelper::new(query),
                error: resp.as_ref().err(),
            });
        resp
    }
}

impl ConnectionSealed for SqliteConnection {}

impl Connection for SqliteConnection {
    type Backend = Sqlite;
    type TransactionManager = AnsiTransactionManager;

    /// Establish a connection to the database specified by `database_url`.
    ///
    /// See [SqliteConnection] for supported `database_url`.
    ///
    /// If the database does not exist, this method will try to
    /// create a new database and then establish a connection to it.
    ///
    /// ## WASM support
    ///
    /// If you plan to use this connection type on the `wasm32-unknown-unknown` target please
    /// make sure to read the following notes:
    ///
    /// * The database is stored in memory by default.
    /// * Persistent VFS (Virtual File Systems) is optional,
    ///   see <https://github.com/Spxg/sqlite-wasm-rs> for details
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
        let statement_use = self.prepared_query(source)?;
        statement_use.run().and_then(|_| {
            self.raw_connection
                .rows_affected_by_last_query()
                .map_err(Error::DeserializationError)
        })
    }

    fn transaction_state(&mut self) -> &mut AnsiTransactionManager
    where
        Self: Sized,
    {
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

impl LoadConnection<DefaultLoadingMode> for SqliteConnection {
    type Cursor<'conn, 'query> = StatementIterator<'conn, 'query>;
    type Row<'conn, 'query> = self::row::SqliteRow<'conn, 'query>;

    fn load<'conn, 'query, T>(
        &'conn mut self,
        source: T,
    ) -> QueryResult<Self::Cursor<'conn, 'query>>
    where
        T: Query + QueryFragment<Self::Backend> + QueryId + 'query,
        Self::Backend: QueryMetadata<T::SqlType>,
    {
        let statement = self.prepared_query(source)?;

        Ok(StatementIterator::new(statement))
    }
}

impl WithMetadataLookup for SqliteConnection {
    fn metadata_lookup(&mut self) -> &mut <Sqlite as TypeMetadata>::MetadataLookup {
        &mut self.metadata_lookup
    }
}

#[cfg(feature = "r2d2")]
impl crate::r2d2::R2D2Connection for crate::sqlite::SqliteConnection {
    fn ping(&mut self) -> QueryResult<()> {
        use crate::RunQueryDsl;

        crate::r2d2::CheckConnectionQuery.execute(self).map(|_| ())
    }

    fn is_broken(&mut self) -> bool {
        AnsiTransactionManager::is_broken_transaction_manager(self)
    }
}

impl MultiConnectionHelper for SqliteConnection {
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

/// The decision returned by an [`on_commit`](SqliteConnection::on_commit)
/// callback, controlling whether a pending commit completes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommitDecision {
    /// Let the commit proceed normally.
    Proceed,
    /// Convert the commit into a rollback.
    Rollback,
}

/// The decision returned by an [`on_progress`](SqliteConnection::on_progress)
/// callback, controlling whether a long-running query keeps executing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProgressDecision {
    /// Let the query continue executing.
    Continue,
    /// Interrupt the query (causes `SQLITE_INTERRUPT`).
    Interrupt,
}

/// The decision returned by an [`on_busy`](SqliteConnection::on_busy)
/// callback when the database is locked.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BusyDecision {
    /// Retry the locked operation.
    Retry,
    /// Give up, returning `SQLITE_BUSY` to the caller.
    GiveUp,
}

impl SqliteConnection {
    /// Run a transaction with `BEGIN IMMEDIATE`
    ///
    /// This method will return an error if a transaction is already open.
    ///
    /// # Example
    ///
    /// ```rust
    /// # include!("../../doctest_setup.rs");
    /// #
    /// # fn main() {
    /// #     run_test().unwrap();
    /// # }
    /// #
    /// # fn run_test() -> QueryResult<()> {
    /// #     let mut conn = SqliteConnection::establish(":memory:").unwrap();
    /// conn.immediate_transaction(|conn| {
    ///     // Do stuff in a transaction
    ///     Ok(())
    /// })
    /// # }
    /// ```
    pub fn immediate_transaction<T, E, F>(&mut self, f: F) -> Result<T, E>
    where
        F: FnOnce(&mut Self) -> Result<T, E>,
        E: From<Error>,
    {
        self.transaction_sql(f, "BEGIN IMMEDIATE")
    }

    /// Run a transaction with `BEGIN EXCLUSIVE`
    ///
    /// This method will return an error if a transaction is already open.
    ///
    /// # Example
    ///
    /// ```rust
    /// # include!("../../doctest_setup.rs");
    /// #
    /// # fn main() {
    /// #     run_test().unwrap();
    /// # }
    /// #
    /// # fn run_test() -> QueryResult<()> {
    /// #     let mut conn = SqliteConnection::establish(":memory:").unwrap();
    /// conn.exclusive_transaction(|conn| {
    ///     // Do stuff in a transaction
    ///     Ok(())
    /// })
    /// # }
    /// ```
    pub fn exclusive_transaction<T, E, F>(&mut self, f: F) -> Result<T, E>
    where
        F: FnOnce(&mut Self) -> Result<T, E>,
        E: From<Error>,
    {
        self.transaction_sql(f, "BEGIN EXCLUSIVE")
    }

    /// Returns the rowid of the most recent successful INSERT on this connection.
    ///
    /// Returns `None` if no successful INSERT into a rowid table has been performed
    /// on this connection, and `Some(rowid)` otherwise.
    ///
    /// See [the SQLite documentation](https://www.sqlite.org/c3ref/last_insert_rowid.html)
    /// for details.
    ///
    /// # Caveats
    /// - Inserts into `WITHOUT ROWID` tables are not recorded
    /// - Failed `INSERT` (constraint violations) do not change the value
    /// - `INSERT OR REPLACE` always updates the value
    /// - Within triggers, returns the rowid of the trigger's INSERT;
    ///   reverts after the trigger completes
    ///
    /// # Example
    /// ```rust
    /// # include!("../../doctest_setup.rs");
    /// # fn main() {
    /// #     run_test().unwrap();
    /// # }
    /// # fn run_test() -> QueryResult<()> {
    /// use core::num::NonZeroI64;
    /// use diesel::connection::SimpleConnection;
    /// let conn = &mut SqliteConnection::establish(":memory:").unwrap();
    /// conn.batch_execute("CREATE TABLE users (id INTEGER PRIMARY KEY, name TEXT NOT NULL)")?;
    /// conn.batch_execute("INSERT INTO users (name) VALUES ('Sean')")?;
    /// let rowid = conn.last_insert_rowid();
    /// assert_eq!(rowid, NonZeroI64::new(1));
    /// conn.batch_execute("INSERT INTO users (name) VALUES ('Tess')")?;
    /// let rowid = conn.last_insert_rowid();
    /// assert_eq!(rowid, NonZeroI64::new(2));
    /// # Ok(())
    /// # }
    /// ```
    pub fn last_insert_rowid(&self) -> Option<NonZeroI64> {
        NonZeroI64::new(self.raw_connection.last_insert_rowid())
    }

    /// Returns an object that can be used to stream a BLOB from the database
    ///
    /// # Example
    ///
    /// ```rust
    /// # include!("../../doctest_setup.rs");
    /// # table! {
    /// #     myblobs {
    /// #         id -> Integer,
    /// #         mydata -> Blob,
    /// #     }
    /// # }
    /// # fn main() {
    /// #     run_test().unwrap();
    /// # }
    /// # fn run_test() -> Result<(), Box<dyn std::error::Error>> {
    /// use std::io::Read;
    /// use diesel::connection::SimpleConnection;
    /// let conn = &mut SqliteConnection::establish(":memory:").unwrap();
    /// conn.batch_execute("CREATE TABLE myblobs (id INTEGER PRIMARY KEY, mydata BLOB)")?;
    /// conn.batch_execute("INSERT INTO myblobs (mydata) VALUES ('abc')")?;
    /// let mut data = conn.get_read_only_blob(myblobs::mydata, 1)?;
    /// let mut buf = vec![];
    /// data.read_to_end(&mut buf)?;
    /// assert_eq!(buf, b"abc");
    /// # Ok(())
    /// # }
    /// ```
    pub fn get_read_only_blob<'conn, 'query, U>(
        &'conn self,
        blob_column: U,
        row_id: i64,
    ) -> Result<sqlite_blob::SqliteReadOnlyBlob<'conn>, Error>
    where
        'query: 'conn,
        U: crate::Column,
        U::Table: nodes::StaticQueryFragment,
        <U::Table as nodes::StaticQueryFragment>::Component: HasDatabaseAndTableName,
    {
        use crate::query_builder::nodes::StaticQueryFragment;
        // this mostly exists for a more natural way to call this function
        let _ = blob_column;

        let database_name = U::Table::STATIC_COMPONENT.database_name().unwrap_or("main");
        let column_name = U::NAME;
        let table_name = U::Table::STATIC_COMPONENT.table_name();

        self.raw_connection
            .blob_open(database_name, table_name, column_name, row_id)
    }

    fn transaction_sql<T, E, F>(&mut self, f: F, sql: &str) -> Result<T, E>
    where
        F: FnOnce(&mut Self) -> Result<T, E>,
        E: From<Error>,
    {
        AnsiTransactionManager::begin_transaction_sql(&mut *self, sql)?;
        match f(&mut *self) {
            Ok(value) => {
                AnsiTransactionManager::commit_transaction(&mut *self)?;
                Ok(value)
            }
            Err(e) => {
                AnsiTransactionManager::rollback_transaction(&mut *self)?;
                Err(e)
            }
        }
    }

    fn prepared_query<'conn, 'query, T>(
        &'conn mut self,
        source: T,
    ) -> QueryResult<StatementUse<'conn, 'query>>
    where
        T: QueryFragment<Sqlite> + QueryId + 'query,
    {
        self.instrumentation
            .on_connection_event(InstrumentationEvent::StartQuery {
                query: &crate::debug_query(&source),
            });
        let raw_connection = &self.raw_connection;
        let cache = &mut self.statement_cache;
        let statement = match cache.cached_statement(
            &source,
            &Sqlite,
            &[],
            raw_connection,
            Statement::prepare,
            &mut *self.instrumentation,
        ) {
            Ok(statement) => statement,
            Err(e) => {
                self.instrumentation
                    .on_connection_event(InstrumentationEvent::FinishQuery {
                        query: &crate::debug_query(&source),
                        error: Some(&e),
                    });

                return Err(e);
            }
        };

        StatementUse::bind(statement, source, &mut *self.instrumentation)
    }

    #[doc(hidden)]
    pub fn register_sql_function<ArgsSqlType, RetSqlType, Args, Ret, F>(
        &mut self,
        fn_name: &str,
        behavior: SqliteFunctionBehavior,
        mut f: F,
    ) -> QueryResult<()>
    where
        F: FnMut(Args) -> Ret + core::panic::UnwindSafe + Send + 'static,
        Args: FromSqlRow<ArgsSqlType, Sqlite> + StaticallySizedRow<ArgsSqlType, Sqlite>,
        Ret: ToSql<RetSqlType, Sqlite>,
        Sqlite: HasSqlType<RetSqlType>,
    {
        functions::register(&self.raw_connection, fn_name, behavior, move |_, args| {
            f(args)
        })
    }

    #[doc(hidden)]
    pub fn register_noarg_sql_function<RetSqlType, Ret, F>(
        &mut self,
        fn_name: &str,
        behavior: SqliteFunctionBehavior,
        f: F,
    ) -> QueryResult<()>
    where
        F: FnMut() -> Ret + core::panic::UnwindSafe + Send + 'static,
        Ret: ToSql<RetSqlType, Sqlite>,
        Sqlite: HasSqlType<RetSqlType>,
    {
        functions::register_noargs(&self.raw_connection, fn_name, behavior, f)
    }

    #[doc(hidden)]
    pub fn register_aggregate_function<ArgsSqlType, RetSqlType, Args, Ret, A>(
        &mut self,
        fn_name: &str,
        behavior: SqliteFunctionBehavior,
    ) -> QueryResult<()>
    where
        A: SqliteAggregateFunction<Args, Output = Ret> + 'static + Send + core::panic::UnwindSafe,
        Args: FromSqlRow<ArgsSqlType, Sqlite> + StaticallySizedRow<ArgsSqlType, Sqlite>,
        Ret: ToSql<RetSqlType, Sqlite>,
        Sqlite: HasSqlType<RetSqlType>,
    {
        functions::register_aggregate::<_, _, _, _, A>(&self.raw_connection, fn_name, behavior)
    }

    /// Register a collation function.
    ///
    /// `collation` must always return the same answer given the same inputs.
    /// If `collation` panics and unwinds the stack, the process is aborted, since it is used
    /// across a C FFI boundary, which cannot be unwound across and there is no way to
    /// signal failures via the SQLite interface in this case..
    ///
    /// If the name is already registered it will be overwritten.
    ///
    /// This method will return an error if registering the function fails, either due to an
    /// out-of-memory situation or because a collation with that name already exists and is
    /// currently being used in parallel by a query.
    ///
    /// The collation needs to be specified when creating a table:
    /// `CREATE TABLE my_table ( str TEXT COLLATE MY_COLLATION )`,
    /// where `MY_COLLATION` corresponds to name passed as `collation_name`.
    ///
    /// # Example
    ///
    /// ```rust
    /// # include!("../../doctest_setup.rs");
    /// #
    /// # fn main() {
    /// #     run_test().unwrap();
    /// # }
    /// #
    /// # fn run_test() -> QueryResult<()> {
    /// #     let mut conn = SqliteConnection::establish(":memory:").unwrap();
    /// // sqlite NOCASE only works for ASCII characters,
    /// // this collation allows handling UTF-8 (barring locale differences)
    /// conn.register_collation("RUSTNOCASE", |rhs, lhs| {
    ///     rhs.to_lowercase().cmp(&lhs.to_lowercase())
    /// })
    /// # }
    /// ```
    pub fn register_collation<F>(&mut self, collation_name: &str, collation: F) -> QueryResult<()>
    where
        F: Fn(&str, &str) -> core::cmp::Ordering + Send + 'static + core::panic::UnwindSafe,
    {
        self.raw_connection
            .register_collation_function(collation_name, collation)
    }

    /// Serialize the current SQLite database into a byte buffer.
    ///
    /// The serialized data is identical to the data that would be written to disk if the database
    /// was saved in a file.
    ///
    /// # Returns
    ///
    /// This function returns a byte slice representing the serialized database.
    pub fn serialize_database_to_buffer(&mut self) -> SerializedDatabase {
        self.raw_connection.serialize()
    }

    /// Deserialize an SQLite database from a byte buffer.
    ///
    /// This function takes a byte slice and attempts to deserialize it into a SQLite database.
    /// If successful, the database is loaded into the connection. If the deserialization fails,
    /// an error is returned.
    ///
    /// The database is opened in READONLY mode.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use diesel::sqlite::SerializedDatabase;
    /// # use diesel::sqlite::SqliteConnection;
    /// # use diesel::result::QueryResult;
    /// # use diesel::sql_query;
    /// # use diesel::Connection;
    /// # use diesel::RunQueryDsl;
    /// # fn main() {
    /// let connection = &mut SqliteConnection::establish(":memory:").unwrap();
    ///
    /// sql_query("CREATE TABLE users (id INTEGER PRIMARY KEY, name TEXT, email TEXT)")
    ///     .execute(connection).unwrap();
    /// sql_query("INSERT INTO users (name, email) VALUES ('John Doe', 'john.doe@example.com'), ('Jane Doe', 'jane.doe@example.com')")
    ///     .execute(connection).unwrap();
    ///
    /// // Serialize the database to a byte vector
    /// let serialized_db: SerializedDatabase = connection.serialize_database_to_buffer();
    ///
    /// // Create a new in-memory SQLite database
    /// let connection = &mut SqliteConnection::establish(":memory:").unwrap();
    ///
    /// // Deserialize the byte vector into the new database
    /// connection.deserialize_readonly_database_from_buffer(serialized_db.as_slice()).unwrap();
    /// #
    /// # }
    /// ```
    // TODO: Diesel 3.0 This signature needs to change, we want to expose more options (schema name, readonly)
    // and also ensure that this is not as unsafe as the current construct anymore. Maybe just accept a owned buffer or static pointer
    // only instead? (So `Cow<'static, [u8]>`?)
    #[allow(unsafe_code)]
    pub fn deserialize_readonly_database_from_buffer(&mut self, data: &[u8]) -> QueryResult<()> {
        // we copy the buffer here
        // to make sure the underlying buffer lives as long as the connection
        self.serialized_data.push(data.to_vec());
        let last = self
            .serialized_data
            .last()
            .expect("We literally pushed it above, so it's there");
        unsafe {
            // SAFETY: We store the buffer inside of the connection and we never touch it until
            // we drop the connection
            self.raw_connection.deserialize(last)
        }
    }

    /// Provides temporary access to the raw SQLite database connection handle.
    ///
    /// This method provides a way to access the underlying `sqlite3` pointer,
    /// enabling direct use of the SQLite C API for advanced features that
    /// Diesel does not wrap, such as the [session extension](https://www.sqlite.org/sessionintro.html),
    /// [hooks](https://www.sqlite.org/c3ref/update_hook.html), or other advanced APIs.
    ///
    /// # Why Diesel Doesn't Wrap These APIs
    ///
    /// Certain SQLite features, such as the session extension, are **optional** and only
    /// available when SQLite is compiled with specific flags (e.g., `-DSQLITE_ENABLE_SESSION`
    /// and `-DSQLITE_ENABLE_PREUPDATE_HOOK` for sessions). These compile-time options determine
    /// whether the corresponding C API functions exist in the SQLite library's ABI.
    ///
    /// Because Diesel must work with any SQLite library at runtime—including system-provided
    /// libraries that may lack these optional features—it **cannot safely provide wrappers**
    /// for APIs that may or may not exist. Doing so would either:
    ///
    /// - Cause **linker errors** at compile time if the user's `libsqlite3-sys` wasn't compiled
    ///   with the required flags, or
    /// - Cause **undefined behavior** at runtime if Diesel called functions that don't exist
    ///   in the linked library.
    ///
    /// While feature gates could theoretically solve this problem, Diesel already has an
    /// extensive API surface with many existing feature combinations. Each new feature gate
    /// adds a **combinatorial explosion** of test configurations that must be validated,
    /// making the library increasingly difficult to maintain. Therefore, exposing the raw
    /// connection is the preferred approach for niche SQLite features.
    ///
    /// By exposing the raw connection handle, Diesel allows users who **know** they have
    /// access to a properly configured SQLite build to use these advanced features directly
    /// through their own FFI bindings.
    ///
    /// # Safety
    ///
    /// This method is marked `unsafe` because improper use of the raw connection handle
    /// can lead to undefined behavior. The caller must ensure that:
    ///
    /// - The connection handle is **not closed** during the callback.
    /// - The connection handle is **not stored** beyond the callback's scope.
    /// - Concurrent access rules are respected (SQLite connections are not thread-safe
    ///   unless using serialized threading mode).
    /// - **Transaction state is not modified** — do not execute `BEGIN`, `COMMIT`,
    ///   `ROLLBACK`, or `SAVEPOINT` statements via the raw handle. Diesel's
    ///   [`AnsiTransactionManager`] tracks transaction nesting internally, and
    ///   bypassing it will cause Diesel's view of the transaction state to diverge
    ///   from SQLite's actual state.
    /// - **Diesel's prepared statements are not disturbed** — do not call
    ///   `sqlite3_finalize()` or `sqlite3_reset()` on statements that belong to
    ///   Diesel's `StatementCache`. Doing so will cause use-after-free or
    ///   double-free when Diesel later accesses those statements.
    ///
    /// [`AnsiTransactionManager`]: crate::connection::AnsiTransactionManager
    ///
    /// # Example
    ///
    /// ```rust
    /// use diesel::sqlite::SqliteConnection;
    /// use diesel::Connection;
    ///
    /// let mut conn = SqliteConnection::establish(":memory:").unwrap();
    ///
    /// // SAFETY: We do not close or store the connection handle,
    /// // and we do not modify Diesel-managed state (transactions, cached statements).
    /// let is_valid = unsafe {
    ///     conn.with_raw_connection(|raw_conn| {
    ///         // The raw connection pointer can be passed to SQLite C API functions
    ///         // from your own `libsqlite3-sys` (native) or `sqlite-wasm-rs` (WASM)
    ///         // dependency — for example, `sqlite3_get_autocommit(raw_conn)` or
    ///         // `sqlite3session_create(raw_conn, ...)`.
    ///         !raw_conn.is_null()
    ///     })
    /// };
    /// assert!(is_valid);
    /// ```
    ///
    /// # Platform Notes
    ///
    /// This method works identically on both native and WASM targets. However,
    /// you must depend on the appropriate FFI crate for your target:
    ///
    /// - **Native**: Add `libsqlite3-sys` as a dependency
    /// - **WASM** (`wasm32-unknown-unknown`): Add `sqlite-wasm-rs` as a dependency
    ///
    /// Both crates expose a compatible `sqlite3` type that can be used with the
    /// pointer returned by this method.
    #[allow(unsafe_code)]
    pub unsafe fn with_raw_connection<R, F>(&mut self, f: F) -> R
    where
        F: FnOnce(*mut ffi::sqlite3) -> R,
    {
        f(self.raw_connection.internal_connection.as_ptr())
    }

    /// Runs `f` with a borrowed `SqliteConnection` wrapping `db`, giving SQLite
    /// callbacks the full connection API. Statements prepared during `f` are
    /// finalized on return, but `db` is left open, since SQLite owns it.
    ///
    /// # Safety
    ///
    /// `db` must be a valid `sqlite3` handle that stays open for the duration
    /// of the call.
    #[allow(unsafe_code)]
    pub(crate) unsafe fn with_borrowed_connection<R>(
        db: core::ptr::NonNull<ffi::sqlite3>,
        f: impl FnOnce(&mut SqliteConnection) -> R,
    ) -> R {
        // Tears the borrowed connection down on every exit path, including a
        // panic unwinding out of `f`.
        struct Borrowed(core::mem::ManuallyDrop<SqliteConnection>);

        impl Drop for Borrowed {
            fn drop(&mut self) {
                // SAFETY: `self.0` is not touched again after this take.
                let conn = unsafe { core::mem::ManuallyDrop::take(&mut self.0) };
                let SqliteConnection {
                    statement_cache,
                    raw_connection,
                    ..
                } = conn;
                // Finalize prepared statements, but do not run `RawConnection`'s
                // `Drop`, which would close a handle we do not own.
                drop(statement_cache);
                core::mem::forget(raw_connection);
            }
        }

        let mut conn = Borrowed(core::mem::ManuallyDrop::new(SqliteConnection {
            statement_cache: StatementCache::new(),
            raw_connection: RawConnection::from_ptr(db),
            transaction_state: AnsiTransactionManager::default(),
            metadata_lookup: (),
            instrumentation: DynInstrumentation::default_instrumentation(),
            serialized_data: Vec::new(),
        }));

        let result = f(&mut conn.0);

        // The borrowed connection is discarded without committing or rolling
        // back, so a transaction left open by `f` would leak onto the handle.
        debug_assert!(
            matches!(
                AnsiTransactionManager::transaction_manager_status_mut(&mut *conn.0)
                    .transaction_depth(),
                Ok(None)
            ),
            "callback must not leave an open transaction on the borrowed connection"
        );

        result
    }

    /// Set a runtime limit for this connection, returning its previous value.
    ///
    /// Lowering these limits is a way to harden a connection against untrusted
    /// SQL. See the [SQLite documentation](https://www.sqlite.org/c3ref/limit.html)
    /// for the meaning of each [`SqliteLimit`].
    ///
    /// # Example
    ///
    /// ```rust
    /// # include!("../../doctest_setup.rs");
    /// # fn main() { run_test(); }
    /// # fn run_test() {
    /// use diesel::sqlite::SqliteLimit;
    ///
    /// let mut conn = SqliteConnection::establish(":memory:").unwrap();
    ///
    /// // Cap SQL statement length at 1 KiB, keeping the previous value.
    /// let previous = conn.set_limit(SqliteLimit::SqlLength, 1024);
    /// assert!(previous > 0);
    /// assert_eq!(conn.get_limit(SqliteLimit::SqlLength), 1024);
    /// # }
    /// ```
    pub fn set_limit(&mut self, limit: SqliteLimit, value: i32) -> i32 {
        self.raw_connection.set_limit(limit, value)
    }

    /// Get the current value of a runtime limit for this connection.
    ///
    /// See the [SQLite documentation](https://www.sqlite.org/c3ref/limit.html)
    /// for the meaning of each [`SqliteLimit`].
    ///
    /// # Example
    ///
    /// ```rust
    /// # include!("../../doctest_setup.rs");
    /// # fn main() { run_test(); }
    /// # fn run_test() {
    /// use diesel::sqlite::SqliteLimit;
    ///
    /// let conn = SqliteConnection::establish(":memory:").unwrap();
    /// assert!(conn.get_limit(SqliteLimit::SqlLength) > 0);
    /// # }
    /// ```
    pub fn get_limit(&self, limit: SqliteLimit) -> i32 {
        self.raw_connection.get_limit(limit)
    }

    /// Apply SQLite's recommended limits for hardening against untrusted SQL.
    ///
    /// These are the values from the "Untrusted SQL Inputs" table of SQLite's
    /// [security documentation](https://sqlite.org/security.html). They are
    /// intentionally restrictive, so call [`set_limit`](Self::set_limit)
    /// afterwards to relax any that are too aggressive for your application.
    ///
    /// | Limit | Value |
    /// |-------|-------|
    /// | `Length` | 1,000,000 |
    /// | `SqlLength` | 100,000 |
    /// | `ColumnCount` | 100 |
    /// | `ExprDepth` | 10 |
    /// | `CompoundSelect` | 3 |
    /// | `VdbeOp` | 25,000 |
    /// | `FunctionArg` | 8 |
    /// | `Attached` | 0 |
    /// | `LikePatternLength` | 50 |
    /// | `VariableNumber` | 10 |
    /// | `TriggerDepth` | 10 |
    ///
    /// The table's `PARSER_DEPTH` recommendation is omitted because it is a
    /// compile-time only setting with no runtime `sqlite3_limit()` category.
    /// `WorkerThreads` is left untouched (its default of 0 is already safe).
    ///
    /// # Example
    ///
    /// ```rust
    /// # include!("../../doctest_setup.rs");
    /// # fn main() { run_test(); }
    /// # fn run_test() {
    /// use diesel::sqlite::SqliteLimit;
    ///
    /// let mut conn = SqliteConnection::establish(":memory:").unwrap();
    /// conn.set_recommended_security_limits();
    /// assert_eq!(conn.get_limit(SqliteLimit::SqlLength), 100_000);
    ///
    /// // Relax an individual limit that is too strict for this application.
    /// conn.set_limit(SqliteLimit::VariableNumber, 999);
    /// assert_eq!(conn.get_limit(SqliteLimit::VariableNumber), 999);
    /// # }
    /// ```
    pub fn set_recommended_security_limits(&mut self) {
        self.set_limit(SqliteLimit::Length, SqliteLimit::SAFE_LENGTH_LIMIT);
        self.set_limit(SqliteLimit::SqlLength, SqliteLimit::SAFE_SQL_LENGTH_LIMIT);
        self.set_limit(
            SqliteLimit::ColumnCount,
            SqliteLimit::SAFE_COLUMN_COUNT_LIMIT,
        );
        self.set_limit(SqliteLimit::ExprDepth, SqliteLimit::SAFE_EXPR_DEPTH_LIMIT);
        self.set_limit(
            SqliteLimit::CompoundSelect,
            SqliteLimit::SAFE_COMPOUND_SELECT_LIMIT,
        );
        self.set_limit(SqliteLimit::VdbeOp, SqliteLimit::SAFE_VDBE_OP_LIMIT);
        self.set_limit(
            SqliteLimit::FunctionArg,
            SqliteLimit::SAFE_FUNCTION_ARG_LIMIT,
        );
        self.set_limit(SqliteLimit::Attached, SqliteLimit::SAFE_ATTACHED_LIMIT);
        self.set_limit(
            SqliteLimit::LikePatternLength,
            SqliteLimit::SAFE_LIKE_PATTERN_LENGTH_LIMIT,
        );
        self.set_limit(
            SqliteLimit::VariableNumber,
            SqliteLimit::SAFE_VARIABLE_NUMBER_LIMIT,
        );
        self.set_limit(
            SqliteLimit::TriggerDepth,
            SqliteLimit::SAFE_TRIGGER_DEPTH_LIMIT,
        );
    }

    /// Enable or disable SQLite defensive mode.
    ///
    /// When enabled, defensive mode prevents direct writes to shadow tables
    /// (FTS5, R-Tree, etc.), dangerous PRAGMAs like `writable_schema`,
    /// `sqlite3_deserialize()` from opening unsafe database images, and other
    /// potentially dangerous operations. Enable it for any connection that may
    /// process untrusted data. It is the single most important hardening flag.
    ///
    /// Requires SQLite 3.26.0 or later, otherwise returns an error.
    ///
    /// # Security Hardening Recipe
    ///
    /// ```rust
    /// # include!("../../doctest_setup.rs");
    /// # fn main() {
    /// #     let mut conn = SqliteConnection::establish(":memory:").unwrap();
    /// conn.set_defensive(true).unwrap();
    /// conn.set_trusted_schema(false).unwrap();
    /// conn.set_recommended_security_limits();
    /// # }
    /// ```
    ///
    /// Extension loading is off by default. Enable it only when needed via
    /// [`with_load_extension_enabled`][Self::with_load_extension_enabled]. See
    /// [`set_recommended_security_limits`][Self::set_recommended_security_limits]
    /// to harden the SQLite resource limits as well.
    pub fn set_defensive(&mut self, enabled: bool) -> QueryResult<()> {
        self.raw_connection
            .set_db_config_bool(ffi::SQLITE_DBCONFIG_DEFENSIVE, enabled)
    }

    /// Check if defensive mode is enabled.
    ///
    /// See [`set_defensive`][Self::set_defensive] for details.
    pub fn is_defensive(&self) -> QueryResult<bool> {
        self.raw_connection
            .get_db_config_bool(ffi::SQLITE_DBCONFIG_DEFENSIVE)
    }

    /// Enable or disable trusted schema mode.
    ///
    /// When disabled (untrusted), SQL functions called from schema objects
    /// (views, triggers, CHECK constraints, DEFAULT expressions, generated
    /// columns, expression indexes) are restricted to those marked
    /// [`INNOCUOUS`][crate::sqlite::SqliteFunctionBehavior::INNOCUOUS]. Disable
    /// it when opening database files from untrusted sources, and register your
    /// custom functions with appropriate
    /// [`SqliteFunctionBehavior`][crate::sqlite::SqliteFunctionBehavior] flags.
    ///
    /// Requires SQLite 3.31.0 or later, otherwise returns an error.
    pub fn set_trusted_schema(&mut self, trusted: bool) -> QueryResult<()> {
        self.raw_connection
            .set_db_config_bool(ffi::SQLITE_DBCONFIG_TRUSTED_SCHEMA, trusted)
    }

    /// Check if trusted schema mode is enabled.
    ///
    /// See [`set_trusted_schema`][Self::set_trusted_schema] for details.
    pub fn is_trusted_schema(&self) -> QueryResult<bool> {
        self.raw_connection
            .get_db_config_bool(ffi::SQLITE_DBCONFIG_TRUSTED_SCHEMA)
    }

    /// Runs the given closure with the `load_extension()` SQL function enabled,
    /// disabling it again afterwards.
    ///
    /// This controls the [`load_extension()`](https://www.sqlite.org/lang_corefunc.html#load_extension)
    /// **SQL function**, not the `sqlite3_load_extension()` C API (which Diesel
    /// does not expose). Extension loading is off by default, and scoping it to a
    /// closure keeps the window in which it is enabled as small as possible.
    ///
    /// Requires SQLite 3.13.0 or later, otherwise returns an error. Has no effect
    /// if SQLite was compiled with `SQLITE_OMIT_LOAD_EXTENSION`.
    ///
    /// # Panics
    ///
    /// If `f` panics, extension loading is disabled again before the panic
    /// resumes. no-std builds cannot catch the unwind, so there the flag is
    /// restored only on a normal return.
    ///
    /// # Example
    ///
    /// ```rust
    /// # include!("../../doctest_setup.rs");
    /// # fn main() {
    /// #     let mut conn = SqliteConnection::establish(":memory:").unwrap();
    /// let result: QueryResult<()> = conn.with_load_extension_enabled(|_conn| Ok(()));
    /// result.unwrap();
    /// # }
    /// ```
    pub fn with_load_extension_enabled<R, E>(
        &mut self,
        f: impl FnOnce(&mut Self) -> Result<R, E>,
    ) -> Result<R, E>
    where
        E: From<crate::result::Error>,
    {
        self.set_load_extension_enabled(true)?;

        // On std builds, catch a panic from `f` so extension loading is restored
        // before the panic is resumed. no-std cannot catch unwinding, so there
        // the flag is restored only on a normal return.
        #[cfg(feature = "std")]
        {
            match std::panic::catch_unwind(core::panic::AssertUnwindSafe(|| f(self))) {
                Ok(r) => {
                    self.set_load_extension_enabled(false)?;
                    r
                }
                Err(panic) => {
                    let _ = self.set_load_extension_enabled(false);
                    std::panic::resume_unwind(panic);
                }
            }
        }
        #[cfg(not(feature = "std"))]
        {
            let r = f(self);
            self.set_load_extension_enabled(false)?;
            r
        }
    }

    fn set_load_extension_enabled(&mut self, enabled: bool) -> QueryResult<()> {
        self.raw_connection
            .set_db_config_bool(ffi::SQLITE_DBCONFIG_ENABLE_LOAD_EXTENSION, enabled)
    }

    #[cfg(test)]
    fn is_load_extension_enabled(&self) -> QueryResult<bool> {
        self.raw_connection
            .get_db_config_bool(ffi::SQLITE_DBCONFIG_ENABLE_LOAD_EXTENSION)
    }

    /// Enable or disable the `fts3_tokenizer()` SQL function.
    ///
    /// The [`fts3_tokenizer()`](https://www.sqlite.org/fts3.html#f3tknzr) function
    /// allows overloading the default FTS3/FTS4 tokenizer, which can be exploited
    /// if an attacker can execute arbitrary SQL. Disable it unless you need custom
    /// FTS3 tokenizers.
    ///
    /// Requires SQLite 3.12.0 or later, otherwise returns an error.
    pub fn set_fts3_tokenizer_enabled(&mut self, enabled: bool) -> QueryResult<()> {
        self.raw_connection
            .set_db_config_bool(ffi::SQLITE_DBCONFIG_ENABLE_FTS3_TOKENIZER, enabled)
    }

    /// Check if the `fts3_tokenizer()` SQL function is enabled.
    ///
    /// See [`set_fts3_tokenizer_enabled`][Self::set_fts3_tokenizer_enabled] for details.
    pub fn is_fts3_tokenizer_enabled(&self) -> QueryResult<bool> {
        self.raw_connection
            .get_db_config_bool(ffi::SQLITE_DBCONFIG_ENABLE_FTS3_TOKENIZER)
    }

    /// Enable or disable direct writes to `sqlite_master`.
    ///
    /// When enabled, allows direct modification of the `sqlite_master` table,
    /// which can corrupt the database if misused. Keep it disabled unless you
    /// need to repair or modify the schema directly. Defensive mode
    /// ([`set_defensive`][Self::set_defensive]) also prevents this.
    ///
    /// Requires SQLite 3.28.0 or later, otherwise returns an error.
    pub fn set_writable_schema(&mut self, enabled: bool) -> QueryResult<()> {
        self.raw_connection
            .set_db_config_bool(ffi::SQLITE_DBCONFIG_WRITABLE_SCHEMA, enabled)
    }

    /// Check if direct writes to `sqlite_master` are enabled.
    ///
    /// See [`set_writable_schema`][Self::set_writable_schema] for details.
    pub fn is_writable_schema(&self) -> QueryResult<bool> {
        self.raw_connection
            .get_db_config_bool(ffi::SQLITE_DBCONFIG_WRITABLE_SCHEMA)
    }

    /// Enable or disable ATTACH from creating new database files.
    ///
    /// When disabled, [`ATTACH`](https://www.sqlite.org/lang_attach.html) can only
    /// open existing database files, not create new ones. Disable it where
    /// database file creation should be restricted.
    ///
    /// Requires SQLite 3.49.0 or later, otherwise returns an error.
    pub fn set_attach_create_enabled(&mut self, enabled: bool) -> QueryResult<()> {
        self.raw_connection
            .set_db_config_bool(raw::SQLITE_DBCONFIG_ENABLE_ATTACH_CREATE, enabled)
    }

    /// Check if ATTACH can create new database files.
    ///
    /// See [`set_attach_create_enabled`][Self::set_attach_create_enabled] for details.
    pub fn is_attach_create_enabled(&self) -> QueryResult<bool> {
        self.raw_connection
            .get_db_config_bool(raw::SQLITE_DBCONFIG_ENABLE_ATTACH_CREATE)
    }

    /// Enable or disable ATTACH from opening databases in write mode.
    ///
    /// When disabled, all attached databases are opened as read-only. Disable it
    /// to restrict write access to attached databases.
    ///
    /// Requires SQLite 3.49.0 or later, otherwise returns an error.
    pub fn set_attach_write_enabled(&mut self, enabled: bool) -> QueryResult<()> {
        self.raw_connection
            .set_db_config_bool(raw::SQLITE_DBCONFIG_ENABLE_ATTACH_WRITE, enabled)
    }

    /// Check if ATTACH can open databases in write mode.
    ///
    /// See [`set_attach_write_enabled`][Self::set_attach_write_enabled] for details.
    pub fn is_attach_write_enabled(&self) -> QueryResult<bool> {
        self.raw_connection
            .get_db_config_bool(raw::SQLITE_DBCONFIG_ENABLE_ATTACH_WRITE)
    }

    /// Enable or disable trigger execution.
    ///
    /// When disabled, triggers will not fire for any DML operations.
    ///
    /// Requires SQLite 3.8.7 or later, otherwise returns an error.
    pub fn set_triggers_enabled(&mut self, enabled: bool) -> QueryResult<()> {
        self.raw_connection
            .set_db_config_bool(ffi::SQLITE_DBCONFIG_ENABLE_TRIGGER, enabled)
    }

    /// Check if trigger execution is enabled.
    ///
    /// See [`set_triggers_enabled`][Self::set_triggers_enabled] for details.
    pub fn are_triggers_enabled(&self) -> QueryResult<bool> {
        self.raw_connection
            .get_db_config_bool(ffi::SQLITE_DBCONFIG_ENABLE_TRIGGER)
    }

    /// Enable or disable view expansion.
    ///
    /// When disabled, queries against views will fail.
    ///
    /// Requires SQLite 3.30.0 or later, otherwise returns an error.
    pub fn set_views_enabled(&mut self, enabled: bool) -> QueryResult<()> {
        self.raw_connection
            .set_db_config_bool(ffi::SQLITE_DBCONFIG_ENABLE_VIEW, enabled)
    }

    /// Check if view expansion is enabled.
    ///
    /// See [`set_views_enabled`][Self::set_views_enabled] for details.
    pub fn are_views_enabled(&self) -> QueryResult<bool> {
        self.raw_connection
            .get_db_config_bool(ffi::SQLITE_DBCONFIG_ENABLE_VIEW)
    }

    /// Enable or disable foreign key constraint enforcement.
    ///
    /// This is equivalent to `PRAGMA foreign_keys = ON/OFF`.
    ///
    /// Requires SQLite 3.8.7 or later, otherwise returns an error.
    pub fn set_foreign_keys_enabled(&mut self, enabled: bool) -> QueryResult<()> {
        self.raw_connection
            .set_db_config_bool(ffi::SQLITE_DBCONFIG_ENABLE_FKEY, enabled)
    }

    /// Check if foreign key constraints are enabled.
    ///
    /// See [`set_foreign_keys_enabled`][Self::set_foreign_keys_enabled] for details.
    pub fn are_foreign_keys_enabled(&self) -> QueryResult<bool> {
        self.raw_connection
            .get_db_config_bool(ffi::SQLITE_DBCONFIG_ENABLE_FKEY)
    }

    /// Enable or disable double-quoted strings in DML statements.
    ///
    /// When enabled, double-quoted strings are interpreted as string literals
    /// rather than identifiers, a legacy behavior that can cause issues. Disable
    /// it for stricter SQL compliance.
    ///
    /// Requires SQLite 3.29.0 or later, otherwise returns an error.
    pub fn set_double_quoted_strings_dml(&mut self, enabled: bool) -> QueryResult<()> {
        self.raw_connection
            .set_db_config_bool(ffi::SQLITE_DBCONFIG_DQS_DML, enabled)
    }

    /// Check if double-quoted strings in DML are enabled.
    ///
    /// See [`set_double_quoted_strings_dml`][Self::set_double_quoted_strings_dml] for details.
    pub fn are_double_quoted_strings_dml_enabled(&self) -> QueryResult<bool> {
        self.raw_connection
            .get_db_config_bool(ffi::SQLITE_DBCONFIG_DQS_DML)
    }

    /// Enable or disable double-quoted strings in DDL statements.
    ///
    /// When enabled, double-quoted strings are interpreted as string literals
    /// rather than identifiers, a legacy behavior that can cause issues. Disable
    /// it for stricter SQL compliance.
    ///
    /// Requires SQLite 3.29.0 or later, otherwise returns an error.
    pub fn set_double_quoted_strings_ddl(&mut self, enabled: bool) -> QueryResult<()> {
        self.raw_connection
            .set_db_config_bool(ffi::SQLITE_DBCONFIG_DQS_DDL, enabled)
    }

    /// Check if double-quoted strings in DDL are enabled.
    ///
    /// See [`set_double_quoted_strings_ddl`][Self::set_double_quoted_strings_ddl] for details.
    pub fn are_double_quoted_strings_ddl_enabled(&self) -> QueryResult<bool> {
        self.raw_connection
            .get_db_config_bool(ffi::SQLITE_DBCONFIG_DQS_DDL)
    }

    fn register_diesel_sql_functions(&self) -> QueryResult<()> {
        use crate::sql_types::{Integer, Text};

        // This function has side effects (creates triggers), so it should not
        // be deterministic. We use DIRECTONLY to prevent it from being called
        // from malicious schema objects in untrusted databases.
        functions::register::<Text, Integer, _, _, _>(
            &self.raw_connection,
            "diesel_manage_updated_at",
            SqliteFunctionBehavior::DIRECTONLY,
            |conn, table_name: String| {
                conn.exec(&alloc::format!(
                    include_str!("diesel_manage_updated_at.sql"),
                    table_name = table_name
                ))
                .expect("Failed to create trigger");
                0 // have to return *something*
            },
        )
    }

    fn establish_inner(database_url: &str) -> Result<SqliteConnection, ConnectionError> {
        use crate::result::ConnectionError::CouldntSetupConfiguration;
        let raw_connection = RawConnection::establish(database_url)?;
        let conn = Self {
            statement_cache: StatementCache::new(),
            raw_connection,
            transaction_state: AnsiTransactionManager::default(),
            metadata_lookup: (),
            instrumentation: DynInstrumentation::none(),
            serialized_data: Vec::new(),
        };
        conn.register_diesel_sql_functions()
            .map_err(CouldntSetupConfiguration)?;
        Ok(conn)
    }
}

fn error_message(err_code: libc::c_int) -> &'static str {
    ffi::code_to_str(err_code)
}

mod private {
    #[doc(hidden)]
    pub trait HasDatabaseAndTableName {
        fn database_name(&self) -> Option<&'static str>;
        fn table_name(&self) -> &'static str;
    }

    impl HasDatabaseAndTableName for crate::query_builder::nodes::Identifier<'static> {
        fn database_name(&self) -> Option<&'static str> {
            None
        }

        fn table_name(&self) -> &'static str {
            self.0
        }
    }

    impl<M> HasDatabaseAndTableName
        for crate::query_builder::nodes::InfixNode<
            crate::query_builder::nodes::Identifier<'static>,
            crate::query_builder::nodes::Identifier<'static>,
            M,
        >
    {
        fn database_name(&self) -> Option<&'static str> {
            Some(self.lhs.0)
        }

        fn table_name(&self) -> &'static str {
            self.rhs.0
        }
    }
}
pub(crate) use self::private::HasDatabaseAndTableName;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dsl::sql;
    use crate::prelude::*;
    use crate::sql_types::{Integer, Text};
    use crate::sqlite::SqliteFunctionBehavior;

    fn connection() -> SqliteConnection {
        SqliteConnection::establish(":memory:").unwrap()
    }

    #[diesel_test_helper::test]
    #[allow(unsafe_code)]
    fn with_raw_connection_can_return_values() {
        let connection = &mut connection();

        // SAFETY: We only read connection status, which doesn't modify state.
        let autocommit_status = unsafe {
            connection.with_raw_connection(|raw_conn| ffi::sqlite3_get_autocommit(raw_conn))
        };

        // Outside a transaction, autocommit should be enabled (returns non-zero)
        assert_ne!(autocommit_status, 0, "Expected autocommit to be enabled");
    }

    #[diesel_test_helper::test]
    #[allow(unsafe_code)]
    fn with_raw_connection_works_after_diesel_operations() {
        let connection = &mut connection();

        // First, do some Diesel operations
        crate::sql_query("CREATE TABLE test_table (id INTEGER PRIMARY KEY, value TEXT)")
            .execute(connection)
            .unwrap();
        crate::sql_query("INSERT INTO test_table (value) VALUES ('hello')")
            .execute(connection)
            .unwrap();

        // SAFETY: We only read the last insert rowid, which is a read-only operation.
        let last_rowid = unsafe {
            connection.with_raw_connection(|raw_conn| ffi::sqlite3_last_insert_rowid(raw_conn))
        };

        assert_eq!(last_rowid, 1, "Last insert rowid should be 1");

        // Verify Diesel still works after using raw connection
        let count: i64 = sql::<crate::sql_types::BigInt>("SELECT COUNT(*) FROM test_table")
            .get_result(connection)
            .unwrap();
        assert_eq!(count, 1);
    }

    #[diesel_test_helper::test]
    #[allow(unsafe_code)]
    fn with_raw_connection_can_execute_raw_sql() {
        let connection = &mut connection();

        // Create a table using Diesel first
        crate::sql_query("CREATE TABLE raw_test (id INTEGER PRIMARY KEY, name TEXT)")
            .execute(connection)
            .unwrap();

        // SAFETY: We execute a simple INSERT via raw SQLite API.
        // This modifies the database but in a way compatible with Diesel.
        let result = unsafe {
            connection.with_raw_connection(|raw_conn| {
                let sql = c"INSERT INTO raw_test (name) VALUES ('from_raw')";
                let mut err_msg: *mut libc::c_char = core::ptr::null_mut();
                let rc = ffi::sqlite3_exec(
                    raw_conn,
                    sql.as_ptr(),
                    None,
                    core::ptr::null_mut(),
                    &mut err_msg,
                );
                if rc != ffi::SQLITE_OK && !err_msg.is_null() {
                    ffi::sqlite3_free(err_msg as *mut libc::c_void);
                }
                rc
            })
        };

        assert_eq!(result, ffi::SQLITE_OK, "Raw SQL execution should succeed");

        // Verify the insert worked using Diesel
        let count: i64 = sql::<crate::sql_types::BigInt>("SELECT COUNT(*) FROM raw_test")
            .get_result(connection)
            .unwrap();
        assert_eq!(count, 1);

        let name: String = sql::<Text>("SELECT name FROM raw_test WHERE id = 1")
            .get_result(connection)
            .unwrap();
        assert_eq!(name, "from_raw");
    }

    #[diesel_test_helper::test]
    #[allow(unsafe_code)]
    fn with_raw_connection_works_within_transaction() {
        let connection = &mut connection();

        crate::sql_query("CREATE TABLE txn_test (id INTEGER PRIMARY KEY, value INTEGER)")
            .execute(connection)
            .unwrap();

        connection
            .transaction::<_, crate::result::Error, _>(|conn| {
                crate::sql_query("INSERT INTO txn_test (value) VALUES (42)")
                    .execute(conn)
                    .unwrap();

                // SAFETY: We only read the autocommit status inside a transaction.
                let autocommit = unsafe {
                    conn.with_raw_connection(|raw_conn| ffi::sqlite3_get_autocommit(raw_conn))
                };

                // Inside a transaction, autocommit should be disabled (returns 0)
                assert_eq!(
                    autocommit, 0,
                    "Autocommit should be disabled inside transaction"
                );

                Ok(())
            })
            .unwrap();

        // After transaction commits, autocommit should be re-enabled
        let autocommit = unsafe {
            connection.with_raw_connection(|raw_conn| ffi::sqlite3_get_autocommit(raw_conn))
        };
        assert_ne!(
            autocommit, 0,
            "Autocommit should be enabled after transaction"
        );
    }

    #[diesel_test_helper::test]
    #[allow(unsafe_code)]
    fn with_raw_connection_can_read_database_filename() {
        let connection = &mut connection();

        // SAFETY: We only read the database filename, which is a read-only operation.
        let filename = unsafe {
            connection.with_raw_connection(|raw_conn| {
                let db_name = c"main";
                let filename_ptr = ffi::sqlite3_db_filename(raw_conn, db_name.as_ptr());
                if filename_ptr.is_null() {
                    None
                } else {
                    // For :memory: databases, this might return empty string or special value
                    let cstr = core::ffi::CStr::from_ptr(filename_ptr);
                    Some(cstr.to_string_lossy().into_owned())
                }
            })
        };

        // For in-memory databases, sqlite3_db_filename returns a non-null pointer
        // to an empty string
        assert_eq!(
            filename,
            Some(String::new()),
            "In-memory database filename should be an empty string"
        );
    }

    #[diesel_test_helper::test]
    #[allow(unsafe_code)]
    fn with_raw_connection_changes_count() {
        let connection = &mut connection();

        crate::sql_query("CREATE TABLE changes_test (id INTEGER PRIMARY KEY, value INTEGER)")
            .execute(connection)
            .unwrap();

        crate::sql_query("INSERT INTO changes_test (value) VALUES (1), (2), (3)")
            .execute(connection)
            .unwrap();

        // Update all rows using raw connection
        let changes = unsafe {
            connection.with_raw_connection(|raw_conn| {
                let sql = c"UPDATE changes_test SET value = value + 10";
                let mut err_msg: *mut libc::c_char = core::ptr::null_mut();
                let rc = ffi::sqlite3_exec(
                    raw_conn,
                    sql.as_ptr(),
                    None,
                    core::ptr::null_mut(),
                    &mut err_msg,
                );
                if rc != ffi::SQLITE_OK && !err_msg.is_null() {
                    ffi::sqlite3_free(err_msg as *mut libc::c_void);
                    return -1;
                }
                ffi::sqlite3_changes(raw_conn)
            })
        };

        assert_eq!(changes, 3, "Should have updated 3 rows");

        // Verify the updates using Diesel
        let values: Vec<i32> = sql::<Integer>("SELECT value FROM changes_test ORDER BY id")
            .load(connection)
            .unwrap();
        assert_eq!(values, vec![11, 12, 13]);
    }

    // catch_unwind is not available in WASM (panic = "abort")
    #[diesel_test_helper::test]
    #[allow(unsafe_code)]
    #[cfg(not(all(target_family = "wasm", target_os = "unknown")))]
    fn with_raw_connection_recovers_after_panic() {
        let connection = &mut connection();

        crate::sql_query("CREATE TABLE panic_test (id INTEGER PRIMARY KEY, value TEXT)")
            .execute(connection)
            .unwrap();

        // Panic inside the callback
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| unsafe {
            connection.with_raw_connection(|_raw_conn| {
                panic!("intentional panic inside with_raw_connection");
            })
        }));
        assert!(result.is_err(), "Should have caught the panic");

        // Connection should still be usable after the panic
        crate::sql_query("INSERT INTO panic_test (value) VALUES ('after_panic')")
            .execute(connection)
            .unwrap();

        let count: i64 = sql::<crate::sql_types::BigInt>("SELECT COUNT(*) FROM panic_test")
            .get_result(connection)
            .unwrap();
        assert_eq!(count, 1, "Connection should work after panic in callback");
    }

    // Filesystem access is not available in WASM
    #[diesel_test_helper::test]
    #[allow(unsafe_code)]
    #[cfg(not(all(target_family = "wasm", target_os = "unknown")))]
    fn with_raw_connection_can_read_file_database_filename() {
        let dir = std::env::temp_dir().join("diesel_test_filename.db");
        let db_path = dir.to_str().unwrap();

        // Clean up from any previous run
        let _ = std::fs::remove_file(db_path);

        let connection = &mut SqliteConnection::establish(db_path).unwrap();

        // SAFETY: We only read the database filename, which is a read-only operation.
        let filename = unsafe {
            connection.with_raw_connection(|raw_conn| {
                let db_name = c"main";
                let filename_ptr = ffi::sqlite3_db_filename(raw_conn, db_name.as_ptr());
                if filename_ptr.is_null() {
                    None
                } else {
                    let cstr = core::ffi::CStr::from_ptr(filename_ptr);
                    Some(cstr.to_string_lossy().into_owned())
                }
            })
        };

        let filename = filename.expect("File-based database should have a filename");
        assert!(
            filename.contains("diesel_test_filename.db"),
            "Filename should contain the database name, got: {filename}"
        );

        // Clean up
        let _ = std::fs::remove_file(db_path);
    }

    #[declare_sql_function]
    extern "SQL" {
        fn fun_case(x: Text) -> Text;
        fn my_add(x: Integer, y: Integer) -> Integer;
        fn answer() -> Integer;
        fn add_counter(x: Integer) -> Integer;

        #[aggregate]
        fn my_sum(expr: Integer) -> Integer;
        #[aggregate]
        fn range_max(expr1: Integer, expr2: Integer, expr3: Integer) -> Nullable<Integer>;
    }

    #[diesel_test_helper::test]
    fn database_serializes_and_deserializes_successfully() {
        let expected_users = vec![
            (
                1,
                "John Doe".to_string(),
                "john.doe@example.com".to_string(),
            ),
            (
                2,
                "Jane Doe".to_string(),
                "jane.doe@example.com".to_string(),
            ),
        ];

        let conn1 = &mut connection();
        let _ =
            crate::sql_query("CREATE TABLE users (id INTEGER PRIMARY KEY, name TEXT, email TEXT)")
                .execute(conn1);
        let _ = crate::sql_query("INSERT INTO users (name, email) VALUES ('John Doe', 'john.doe@example.com'), ('Jane Doe', 'jane.doe@example.com')")
            .execute(conn1);

        for _i in 0..2 {
            let serialized_database = conn1.serialize_database_to_buffer();
            let conn2 = &mut connection();
            conn2
                .deserialize_readonly_database_from_buffer(serialized_database.as_slice())
                .unwrap();

            let query =
                sql::<(Integer, Text, Text)>("SELECT id, name, email FROM users ORDER BY id");
            let actual_users = query.load::<(i32, String, String)>(conn2).unwrap();

            assert_eq!(expected_users, actual_users);
            // drop the database here
            // and requery the database to make sure the database owns
            // required data
            std::mem::drop(serialized_database);
            let query =
                sql::<(Integer, Text, Text)>("SELECT id, name, email FROM users ORDER BY id");
            let actual_users = query.load::<(i32, String, String)>(conn2).unwrap();

            assert_eq!(expected_users, actual_users);
        }
    }

    #[diesel_test_helper::test]
    fn database_deserialize_random_bytes() {
        let buffer = vec![0, 1, 2, 3, 4];
        let conn = &mut SqliteConnection::establish(":memory:").unwrap();

        conn.deserialize_readonly_database_from_buffer(&buffer)
            .unwrap();

        let r = sql::<Integer>("SELECT id FROM users").load::<i32>(conn);

        assert!(r.is_err());
        assert_eq!(r.unwrap_err().to_string(), "file is not a database");

        let conn = &mut SqliteConnection::establish(":memory:").unwrap();

        let _ =
            crate::sql_query("CREATE TABLE users (id INTEGER PRIMARY KEY, name TEXT, email TEXT)")
                .execute(conn);
        let _ = crate::sql_query("INSERT INTO users (name, email) VALUES ('John Doe', 'john.doe@example.com'), ('Jane Doe', 'jane.doe@example.com')")
            .execute(conn);

        let db = conn.serialize_database_to_buffer();
        // only get a valid header, but append garbage
        let mut bad_buffer = db[..100].to_vec();
        bad_buffer.extend(b"whatever");
        conn.deserialize_readonly_database_from_buffer(&bad_buffer)
            .unwrap();

        let r = sql::<Integer>("SELECT id FROM users").load::<i32>(conn);

        assert!(r.is_err());
        assert_eq!(
            r.unwrap_err().to_string(),
            "database disk image is malformed"
        );

        // only get a valid header, but append garbage
        let mut size_fitting_bad_buffer = db[..100].to_vec();
        size_fitting_bad_buffer.extend(
            core::iter::repeat(b"abcdefghij")
                .flatten()
                .take(db.len() - 100),
        );
        let r = conn.deserialize_readonly_database_from_buffer(&size_fitting_bad_buffer);

        assert!(r.is_err());
        assert_eq!(
            r.unwrap_err().to_string(),
            "database disk image is malformed"
        );
    }

    #[diesel_test_helper::test]
    fn register_custom_function() {
        let connection = &mut connection();
        fun_case_utils::register_impl(connection, |x: String| {
            x.chars()
                .enumerate()
                .map(|(i, c)| {
                    if i % 2 == 0 {
                        c.to_lowercase().to_string()
                    } else {
                        c.to_uppercase().to_string()
                    }
                })
                .collect::<String>()
        })
        .unwrap();

        let mapped_string = crate::select(fun_case("foobar"))
            .get_result::<String>(connection)
            .unwrap();
        assert_eq!("fOoBaR", mapped_string);
    }

    #[diesel_test_helper::test]
    fn register_multiarg_function() {
        let connection = &mut connection();
        my_add_utils::register_impl(connection, |x: i32, y: i32| x + y).unwrap();

        let added = crate::select(my_add(1, 2)).get_result::<i32>(connection);
        assert_eq!(Ok(3), added);
    }

    #[diesel_test_helper::test]
    fn register_noarg_function() {
        let connection = &mut connection();
        answer_utils::register_impl(connection, || 42).unwrap();

        let answer = crate::select(answer()).get_result::<i32>(connection);
        assert_eq!(Ok(42), answer);
    }

    #[diesel_test_helper::test]
    fn register_nondeterministic_noarg_function() {
        let connection = &mut connection();
        answer_utils::register_nondeterministic_impl(connection, || 42).unwrap();

        let answer = crate::select(answer()).get_result::<i32>(connection);
        assert_eq!(Ok(42), answer);
    }

    #[diesel_test_helper::test]
    fn register_nondeterministic_function() {
        let connection = &mut connection();
        let mut y = 0;
        add_counter_utils::register_nondeterministic_impl(connection, move |x: i32| {
            y += 1;
            x + y
        })
        .unwrap();

        let added = crate::select((add_counter(1), add_counter(1), add_counter(1)))
            .get_result::<(i32, i32, i32)>(connection);
        assert_eq!(Ok((2, 3, 4)), added);
    }

    #[derive(Default)]
    struct MySum {
        sum: i32,
    }

    impl SqliteAggregateFunction<i32> for MySum {
        type Output = i32;

        fn step(&mut self, expr: i32) {
            self.sum += expr;
        }

        fn finalize(aggregator: Option<Self>) -> Self::Output {
            aggregator.map(|a| a.sum).unwrap_or_default()
        }
    }

    table! {
        my_sum_example {
            id -> Integer,
            value -> Integer,
        }
    }

    #[diesel_test_helper::test]
    fn register_aggregate_function() {
        use self::my_sum_example::dsl::*;

        let connection = &mut connection();
        crate::sql_query(
            "CREATE TABLE my_sum_example (id integer primary key autoincrement, value integer)",
        )
        .execute(connection)
        .unwrap();
        crate::sql_query("INSERT INTO my_sum_example (value) VALUES (1), (2), (3)")
            .execute(connection)
            .unwrap();

        my_sum_utils::register_impl_with_behavior::<MySum, _>(
            connection,
            SqliteFunctionBehavior::DETERMINISTIC,
        )
        .unwrap();

        let result = my_sum_example
            .select(my_sum(value))
            .get_result::<i32>(connection);
        assert_eq!(Ok(6), result);
    }

    #[diesel_test_helper::test]
    fn register_aggregate_function_returns_finalize_default_on_empty_set() {
        use self::my_sum_example::dsl::*;

        let connection = &mut connection();
        crate::sql_query(
            "CREATE TABLE my_sum_example (id integer primary key autoincrement, value integer)",
        )
        .execute(connection)
        .unwrap();

        my_sum_utils::register_impl_with_behavior::<MySum, _>(
            connection,
            SqliteFunctionBehavior::DETERMINISTIC,
        )
        .unwrap();

        let result = my_sum_example
            .select(my_sum(value))
            .get_result::<i32>(connection);
        assert_eq!(Ok(0), result);
    }

    #[derive(Default)]
    struct RangeMax<T> {
        max_value: Option<T>,
    }

    impl<T: Default + Ord + Copy + Clone> SqliteAggregateFunction<(T, T, T)> for RangeMax<T> {
        type Output = Option<T>;

        fn step(&mut self, (x0, x1, x2): (T, T, T)) {
            let max = if x0 >= x1 && x0 >= x2 {
                x0
            } else if x1 >= x0 && x1 >= x2 {
                x1
            } else {
                x2
            };

            self.max_value = match self.max_value {
                Some(current_max_value) if max > current_max_value => Some(max),
                None => Some(max),
                _ => self.max_value,
            };
        }

        fn finalize(aggregator: Option<Self>) -> Self::Output {
            aggregator?.max_value
        }
    }

    table! {
        range_max_example {
            id -> Integer,
            value1 -> Integer,
            value2 -> Integer,
            value3 -> Integer,
        }
    }

    #[diesel_test_helper::test]
    fn register_aggregate_multiarg_function() {
        use self::range_max_example::dsl::*;

        let connection = &mut connection();
        crate::sql_query(
            r#"CREATE TABLE range_max_example (
                id integer primary key autoincrement,
                value1 integer,
                value2 integer,
                value3 integer
            )"#,
        )
        .execute(connection)
        .unwrap();
        crate::sql_query(
            "INSERT INTO range_max_example (value1, value2, value3) VALUES (3, 2, 1), (2, 2, 2)",
        )
        .execute(connection)
        .unwrap();

        range_max_utils::register_impl_with_behavior::<RangeMax<i32>, _, _, _>(
            connection,
            SqliteFunctionBehavior::DETERMINISTIC,
        )
        .unwrap();
        let result = range_max_example
            .select(range_max(value1, value2, value3))
            .get_result::<Option<i32>>(connection)
            .unwrap();
        assert_eq!(Some(3), result);
    }

    table! {
        my_collation_example {
            id -> Integer,
            value -> Text,
        }
    }

    #[diesel_test_helper::test]
    fn register_collation_function() {
        use self::my_collation_example::dsl::*;

        let connection = &mut connection();

        connection
            .register_collation("RUSTNOCASE", |rhs, lhs| {
                rhs.to_lowercase().cmp(&lhs.to_lowercase())
            })
            .unwrap();

        crate::sql_query(
                "CREATE TABLE my_collation_example (id integer primary key autoincrement, value text collate RUSTNOCASE)",
            ).execute(connection)
            .unwrap();
        crate::sql_query(
            "INSERT INTO my_collation_example (value) VALUES ('foo'), ('FOo'), ('f00')",
        )
        .execute(connection)
        .unwrap();

        let result = my_collation_example
            .filter(value.eq("foo"))
            .select(value)
            .load::<String>(connection);
        assert_eq!(
            Ok(&["foo".to_owned(), "FOo".to_owned()][..]),
            result.as_ref().map(|vec| vec.as_ref())
        );

        let result = my_collation_example
            .filter(value.eq("FOO"))
            .select(value)
            .load::<String>(connection);
        assert_eq!(
            Ok(&["foo".to_owned(), "FOo".to_owned()][..]),
            result.as_ref().map(|vec| vec.as_ref())
        );

        let result = my_collation_example
            .filter(value.eq("f00"))
            .select(value)
            .load::<String>(connection);
        assert_eq!(
            Ok(&["f00".to_owned()][..]),
            result.as_ref().map(|vec| vec.as_ref())
        );

        let result = my_collation_example
            .filter(value.eq("F00"))
            .select(value)
            .load::<String>(connection);
        assert_eq!(
            Ok(&["f00".to_owned()][..]),
            result.as_ref().map(|vec| vec.as_ref())
        );

        let result = my_collation_example
            .filter(value.eq("oof"))
            .select(value)
            .load::<String>(connection);
        assert_eq!(Ok(&[][..]), result.as_ref().map(|vec| vec.as_ref()));
    }

    // regression test for https://github.com/diesel-rs/diesel/issues/3425
    #[diesel_test_helper::test]
    fn test_correct_serialization_of_owned_strings() {
        use crate::prelude::*;

        #[derive(Debug, crate::expression::AsExpression)]
        #[diesel(sql_type = diesel::sql_types::Text)]
        struct CustomWrapper(String);

        impl crate::serialize::ToSql<Text, Sqlite> for CustomWrapper {
            fn to_sql<'b>(
                &'b self,
                out: &mut crate::serialize::Output<'b, '_, Sqlite>,
            ) -> crate::serialize::Result {
                out.set_value(self.0.to_string());
                Ok(crate::serialize::IsNull::No)
            }
        }

        let connection = &mut connection();

        let res = crate::select(
            CustomWrapper("".into())
                .into_sql::<crate::sql_types::Text>()
                .nullable(),
        )
        .get_result::<Option<String>>(connection)
        .unwrap();
        assert_eq!(res, Some(String::new()));
    }

    #[diesel_test_helper::test]
    fn test_correct_serialization_of_owned_bytes() {
        use crate::prelude::*;

        #[derive(Debug, crate::expression::AsExpression)]
        #[diesel(sql_type = diesel::sql_types::Binary)]
        struct CustomWrapper(Vec<u8>);

        impl crate::serialize::ToSql<crate::sql_types::Binary, Sqlite> for CustomWrapper {
            fn to_sql<'b>(
                &'b self,
                out: &mut crate::serialize::Output<'b, '_, Sqlite>,
            ) -> crate::serialize::Result {
                out.set_value(self.0.clone());
                Ok(crate::serialize::IsNull::No)
            }
        }

        let connection = &mut connection();

        let res = crate::select(
            CustomWrapper(Vec::new())
                .into_sql::<crate::sql_types::Binary>()
                .nullable(),
        )
        .get_result::<Option<Vec<u8>>>(connection)
        .unwrap();
        assert_eq!(res, Some(Vec::new()));
    }

    #[diesel_test_helper::test]
    fn correctly_handle_empty_query() {
        let check_empty_query_error = |r: crate::QueryResult<usize>| {
            assert!(r.is_err());
            let err = r.unwrap_err();
            assert!(
                matches!(err, crate::result::Error::QueryBuilderError(ref b) if b.is::<crate::result::EmptyQuery>()),
                "Expected a query builder error, but got {err}"
            );
        };
        let connection = &mut SqliteConnection::establish(":memory:").unwrap();
        check_empty_query_error(crate::sql_query("").execute(connection));
        check_empty_query_error(crate::sql_query("   ").execute(connection));
        check_empty_query_error(crate::sql_query("\n\t").execute(connection));
        check_empty_query_error(crate::sql_query("-- SELECT 1;").execute(connection));
    }

    #[diesel_test_helper::test]
    fn last_insert_rowid_returns_none_on_fresh_connection() {
        let conn = &mut connection();
        assert_eq!(conn.last_insert_rowid(), None);
    }

    #[diesel_test_helper::test]
    fn last_insert_rowid_returns_rowid_after_insert() {
        let conn = &mut connection();
        crate::sql_query("CREATE TABLE li_test (id INTEGER PRIMARY KEY, val TEXT NOT NULL)")
            .execute(conn)
            .unwrap();

        crate::sql_query("INSERT INTO li_test (val) VALUES ('a')")
            .execute(conn)
            .unwrap();
        assert_eq!(conn.last_insert_rowid(), NonZeroI64::new(1));

        crate::sql_query("INSERT INTO li_test (val) VALUES ('b')")
            .execute(conn)
            .unwrap();
        assert_eq!(conn.last_insert_rowid(), NonZeroI64::new(2));
    }

    #[diesel_test_helper::test]
    fn last_insert_rowid_unchanged_after_failed_insert() {
        let conn = &mut connection();
        crate::sql_query(
            "CREATE TABLE li_test2 (id INTEGER PRIMARY KEY, val TEXT NOT NULL UNIQUE)",
        )
        .execute(conn)
        .unwrap();

        crate::sql_query("INSERT INTO li_test2 (val) VALUES ('a')")
            .execute(conn)
            .unwrap();
        let rowid = conn.last_insert_rowid();
        assert_eq!(rowid, NonZeroI64::new(1));

        // This should fail due to UNIQUE constraint
        let result = crate::sql_query("INSERT INTO li_test2 (val) VALUES ('a')").execute(conn);
        assert!(result.is_err());

        // rowid should be unchanged
        assert_eq!(conn.last_insert_rowid(), NonZeroI64::new(1));
    }

    #[diesel_test_helper::test]
    fn last_insert_rowid_with_explicit_rowid() {
        let conn = &mut connection();
        crate::sql_query("CREATE TABLE li_test3 (id INTEGER PRIMARY KEY, val TEXT NOT NULL)")
            .execute(conn)
            .unwrap();

        crate::sql_query("INSERT INTO li_test3 (id, val) VALUES (42, 'a')")
            .execute(conn)
            .unwrap();
        assert_eq!(conn.last_insert_rowid(), NonZeroI64::new(42));
    }

    #[diesel_test_helper::test]
    fn last_insert_rowid_unchanged_after_delete_and_update() {
        let conn = &mut connection();
        crate::sql_query("CREATE TABLE li_test4 (id INTEGER PRIMARY KEY, val TEXT NOT NULL)")
            .execute(conn)
            .unwrap();

        crate::sql_query("INSERT INTO li_test4 (val) VALUES ('a')")
            .execute(conn)
            .unwrap();
        let rowid = conn.last_insert_rowid();
        assert_eq!(rowid, NonZeroI64::new(1));

        crate::sql_query("UPDATE li_test4 SET val = 'b' WHERE id = 1")
            .execute(conn)
            .unwrap();
        assert_eq!(conn.last_insert_rowid(), NonZeroI64::new(1));

        crate::sql_query("DELETE FROM li_test4 WHERE id = 1")
            .execute(conn)
            .unwrap();
        assert_eq!(conn.last_insert_rowid(), NonZeroI64::new(1));
    }

    #[diesel_test_helper::test]
    fn read_bytes_from_blob() {
        table! {
            blobs {
                id -> Integer,
                data -> Blob,
                data2 -> Blob,
            }
        }

        use std::io::Read;

        let conn = &mut connection();

        let _ =
            crate::sql_query("CREATE TABLE blobs (id INTEGER PRIMARY KEY, data BLOB, data2 BLOB)")
                .execute(conn);

        let _ = crate::sql_query(
            "INSERT INTO blobs (data, data2) VALUES ('abc', 'def'), ('123', '456')",
        )
        .execute(conn);

        let mut data = conn.get_read_only_blob(blobs::data, 1).unwrap();
        let mut buf = vec![];
        data.read_to_end(&mut buf).unwrap();

        assert_eq!(buf, b"abc");

        let mut data2 = conn.get_read_only_blob(blobs::data2, 1).unwrap();
        let mut buf = vec![];
        data2.read_to_end(&mut buf).unwrap();

        assert_eq!(buf, b"def");
    }

    #[diesel_test_helper::test]
    fn read_seek_bytes() {
        table! {
            blobs {
                id -> Integer,
                data -> Blob,
            }
        }

        use std::io::Read;
        use std::io::Seek;
        use std::io::SeekFrom;

        let conn = &mut connection();

        let _ = crate::sql_query("CREATE TABLE blobs (id INTEGER PRIMARY KEY, data BLOB)")
            .execute(conn);

        let _ = crate::sql_query("INSERT INTO blobs (data) VALUES ('abcdefghi')").execute(conn);

        let mut data = conn.get_read_only_blob(blobs::data, 1).unwrap();

        let mut buf = [0; 1];
        assert_eq!(data.read(&mut buf).unwrap(), 1);
        assert_eq!(&buf, b"a");

        // Seek one forward
        assert_eq!(data.seek(SeekFrom::Current(1)).unwrap(), 2);

        let mut buf = [0; 1];
        assert_eq!(data.read(&mut buf).unwrap(), 1);
        assert_eq!(&buf, b"c");

        // Seek back to start
        assert_eq!(data.seek(SeekFrom::Start(0)).unwrap(), 0);

        let mut buf = [0; 1];
        assert_eq!(data.read(&mut buf).unwrap(), 1);
        assert_eq!(&buf, b"a");

        // Seek before start
        assert_eq!(data.seek(SeekFrom::Current(-10)).unwrap(), 0);

        let mut buf = [0; 1];
        assert_eq!(data.read(&mut buf).unwrap(), 1);
        assert_eq!(&buf, b"a");

        // Seek after end
        data.seek(SeekFrom::Current(100)).unwrap();

        // Now we don't get any bytes back
        let mut buf = [0; 1];
        assert_eq!(data.read(&mut buf).unwrap(), 0);
    }

    #[diesel_test_helper::test]
    fn use_conn_after_blob_drop() {
        table! {
            blobs {
                id -> Integer,
                data -> Blob,
            }
        }

        let conn = &mut connection();

        let _ = crate::sql_query("CREATE TABLE blobs (id INTEGER PRIMARY KEY, data BLOB)")
            .execute(conn);

        let _ = crate::sql_query("INSERT INTO blobs (data) VALUES ('abc')").execute(conn);

        let data = conn.get_read_only_blob(blobs::data, 1).unwrap();
        drop(data);

        let _ = crate::sql_query("INSERT INTO blobs (data) VALUES ('def')").execute(conn);
    }

    #[diesel_test_helper::test]
    fn blob_transaction() {
        table! {
            blobs {
                id -> Integer,
                data -> Blob,
            }
        }

        use std::io::Read;

        let conn = &mut connection();

        let _ = crate::sql_query("CREATE TABLE blobs (id INTEGER PRIMARY KEY, data BLOB)")
            .execute(conn);

        let _ = crate::sql_query("INSERT INTO blobs (data) VALUES ('abc')").execute(conn);

        {
            let mut data = conn.get_read_only_blob(blobs::data, 1).unwrap();
            let mut buf = vec![];
            data.read_to_end(&mut buf).unwrap();
            assert_eq!(buf, b"abc");
        }

        let res = conn.exclusive_transaction(|conn| {
            crate::sql_query("UPDATE blobs SET data = 'def' WHERE id = 1").execute(conn)?;

            let mut data = conn.get_read_only_blob(blobs::data, 1).unwrap();
            let mut buf = vec![];
            data.read_to_end(&mut buf).unwrap();
            assert_eq!(buf, b"def");

            Result::<(), _>::Err(Error::RollbackTransaction)
        });

        assert_eq!(res.unwrap_err(), Error::RollbackTransaction);

        let mut data = conn.get_read_only_blob(blobs::data, 1).unwrap();
        let mut buf = vec![];
        data.read_to_end(&mut buf).unwrap();
        assert_eq!(buf, b"abc");
    }

    #[diesel_test_helper::test]
    fn aggregate_function_works_with_aligned_data() {
        #[derive(Debug, Default)]
        #[repr(align(64))]
        struct OverAligned;

        impl SqliteAggregateFunction<i32> for OverAligned {
            type Output = i64;

            fn step(&mut self, _value: i32) {
                let need = core::mem::align_of::<Self>();
                let got = core::mem::align_of_val(self);
                assert_eq!(need, got);
            }

            fn finalize(_agg: Option<Self>) -> i64 {
                0
            }
        }
        #[declare_sql_function]
        extern "SQL" {
            #[aggregate]
            fn over_aligned_sum(x: Integer) -> diesel::sql_types::BigInt;
        }

        let mut conn = SqliteConnection::establish(":memory:").unwrap();
        over_aligned_sum_utils::register_impl::<OverAligned, _>(&mut conn).unwrap();

        diesel::select(over_aligned_sum(1))
            .execute(&mut conn)
            .unwrap();
    }

    #[diesel_test_helper::test]
    fn sum_twice() {
        #[derive(Default)]
        struct Sum(i32);

        impl SqliteAggregateFunction<i32> for Sum {
            type Output = i32;

            fn step(&mut self, value: i32) {
                self.0 += value;
            }

            fn finalize(agg: Option<Self>) -> i32 {
                agg.map(|s| s.0).unwrap_or_default()
            }
        }

        #[declare_sql_function]
        extern "SQL" {
            #[aggregate]
            fn my_sum(x: Integer) -> Integer;
        }

        let mut conn = SqliteConnection::establish(":memory:").unwrap();
        my_sum_utils::register_impl::<Sum, _>(&mut conn).unwrap();

        conn.batch_execute(
            "
            CREATE TABLE test(key1 INTEGER, key2 INTEGER);
            INSERT INTO test(key1, key2) VALUES (1, 2), (2, 4), (3, 6);
",
        )
        .unwrap();

        table! {
            test (key1, key2) {
                key1 -> Integer,
                key2 -> Integer,
            }
        }

        let (first_res, second_res) = test::table
            .select((my_sum(test::key1), my_sum(test::key2)))
            .get_result::<(i32, i32)>(&mut conn)
            .unwrap();

        assert_eq!(first_res, 6);
        assert_eq!(second_res, 12);

        conn.batch_execute("DELETE FROM test").unwrap();
        let (first_res, second_res) = test::table
            .select((my_sum(test::key1), my_sum(test::key2)))
            .get_result::<(i32, i32)>(&mut conn)
            .unwrap();

        assert_eq!(first_res, 0);
        assert_eq!(second_res, 0);
    }

    #[diesel_test_helper::test]
    fn test_injection() {
        diesel::table! {
            #[sql_name = "quote'table"]
            quote_table (id) {
                id -> Nullable<Integer>,
                name -> Nullable<Text>,
            }
        }

        let mut conn = SqliteConnection::establish(":memory:").unwrap();

        conn.batch_execute("CREATE TABLE \"quote'table\" (id INTEGER PRIMARY KEY, name TEXT);")
            .unwrap();

        diesel::insert_into(quote_table::table)
            .values((quote_table::id.eq(1), quote_table::name.eq("Jane")))
            .execute(&mut conn)
            .unwrap();

        let data = quote_table::table
            .load::<(Option<i32>, Option<String>)>(&mut conn)
            .unwrap();
        assert_eq!(data, [(Some(1), Some("Jane".to_owned()))]);
    }

    #[diesel_test_helper::test]
    fn set_limit_returns_previous_value() {
        let mut conn = connection();
        let original = conn.get_limit(SqliteLimit::SqlLength);

        // Setting a new value returns the old one, and a second set returns the
        // value installed by the first.
        assert_eq!(conn.set_limit(SqliteLimit::SqlLength, 1024), original);
        assert_eq!(conn.set_limit(SqliteLimit::SqlLength, 2048), 1024);
        assert_eq!(conn.get_limit(SqliteLimit::SqlLength), 2048);
    }

    #[diesel_test_helper::test]
    fn get_limit_does_not_mutate() {
        let conn = connection();
        let first = conn.get_limit(SqliteLimit::ExprDepth);
        // Querying is implemented by passing -1 to sqlite3_limit, which must
        // leave the limit unchanged.
        assert!(first > 0);
        assert_eq!(conn.get_limit(SqliteLimit::ExprDepth), first);
    }

    #[diesel_test_helper::test]
    fn set_limit_enforces_length() {
        let mut conn = connection();
        conn.set_limit(SqliteLimit::Length, 100);

        assert!(
            crate::sql_query("SELECT length(randomblob(50))")
                .execute(&mut conn)
                .is_ok()
        );
        // A 500-byte blob exceeds the 100-byte row/value limit ("string or blob too big").
        assert!(
            crate::sql_query("SELECT length(randomblob(500))")
                .execute(&mut conn)
                .is_err()
        );
    }

    #[diesel_test_helper::test]
    fn set_limit_enforces_column_count() {
        // A wide result set runs under the default column limit but fails once the limit is
        // lowered below its column count ("too many columns in result set").
        let wide = format!(
            "SELECT {}",
            (1..=30)
                .map(|i| i.to_string())
                .collect::<Vec<_>>()
                .join(", ")
        );

        let mut unconstrained = connection();
        assert!(crate::sql_query(&wide).execute(&mut unconstrained).is_ok());

        let mut conn = connection();
        conn.set_limit(SqliteLimit::ColumnCount, 10);
        assert!(crate::sql_query(&wide).execute(&mut conn).is_err());
    }

    #[diesel_test_helper::test]
    fn set_limit_enforces_expr_depth() {
        let mut conn = connection();
        conn.set_limit(SqliteLimit::ExprDepth, 5);

        assert!(crate::sql_query("SELECT 1+1").execute(&mut conn).is_ok());
        // A 40-deep addition tree exceeds the parse-tree depth of five.
        let deep = format!("SELECT {}1", "1+".repeat(40));
        assert!(crate::sql_query(&deep).execute(&mut conn).is_err());
    }

    #[diesel_test_helper::test]
    fn set_limit_enforces_compound_select() {
        let mut conn = connection();
        conn.set_limit(SqliteLimit::CompoundSelect, 2);

        assert!(
            crate::sql_query("SELECT 1 UNION SELECT 2")
                .execute(&mut conn)
                .is_ok()
        );
        // Five UNION terms exceed the limit of two ("too many terms in compound SELECT").
        assert!(
            crate::sql_query(
                "SELECT 1 UNION SELECT 2 UNION SELECT 3 UNION SELECT 4 UNION SELECT 5"
            )
            .execute(&mut conn)
            .is_err()
        );
    }

    #[diesel_test_helper::test]
    fn set_limit_enforces_vdbe_op() {
        // The same heavy statement runs under the default opcode budget but fails once that
        // budget is restricted to a tiny value (reported as SQLITE_NOMEM).
        let heavy = "SELECT count(*) FROM sqlite_master a, sqlite_master b, sqlite_master c";

        let mut unconstrained = connection();
        assert!(crate::sql_query(heavy).execute(&mut unconstrained).is_ok());

        let mut conn = connection();
        conn.set_limit(SqliteLimit::VdbeOp, 5);
        assert!(crate::sql_query(heavy).execute(&mut conn).is_err());
    }

    #[diesel_test_helper::test]
    fn set_limit_enforces_function_arg() {
        let mut conn = connection();
        conn.set_limit(SqliteLimit::FunctionArg, 3);

        assert!(
            crate::sql_query("SELECT max(1, 2, 3)")
                .execute(&mut conn)
                .is_ok()
        );
        // Eight arguments exceed the limit of three ("too many arguments on function max").
        assert!(
            crate::sql_query("SELECT max(1, 2, 3, 4, 5, 6, 7, 8)")
                .execute(&mut conn)
                .is_err()
        );
    }

    #[diesel_test_helper::test]
    fn set_limit_enforces_attached() {
        let mut conn = connection();
        conn.set_limit(SqliteLimit::Attached, 0);

        // With zero attachments allowed, any ATTACH is rejected ("too many attached databases").
        assert!(
            crate::sql_query("ATTACH DATABASE ':memory:' AS aux_db")
                .execute(&mut conn)
                .is_err()
        );
    }

    #[diesel_test_helper::test]
    fn set_limit_enforces_variable_number() {
        let mut conn = connection();
        // The published default sits below the bundled ceiling, so it is applied verbatim and
        // acts as the boundary: a parameter index at the limit is accepted, one past it is
        // rejected ("variable number must be between ?1 and ?N").
        conn.set_limit(
            SqliteLimit::VariableNumber,
            SqliteLimit::DEFAULT_VARIABLE_NUMBER_LIMIT,
        );
        let at_limit = format!("SELECT ?{}", SqliteLimit::DEFAULT_VARIABLE_NUMBER_LIMIT);
        let past_limit = format!(
            "SELECT ?{}",
            SqliteLimit::DEFAULT_VARIABLE_NUMBER_LIMIT as i64 + 1
        );
        assert!(crate::sql_query(&at_limit).execute(&mut conn).is_ok());
        assert!(crate::sql_query(&past_limit).execute(&mut conn).is_err());
    }

    #[diesel_test_helper::test]
    fn set_limit_enforces_trigger_depth() {
        use crate::connection::SimpleConnection;

        // A recursive trigger that terminates on its own at x = 100.
        let setup = "PRAGMA recursive_triggers = ON;\
             CREATE TABLE recur (x INTEGER);\
             CREATE TRIGGER recur_tr AFTER INSERT ON recur WHEN NEW.x < 100 \
             BEGIN INSERT INTO recur VALUES (NEW.x + 1); END;";

        // Under the default depth the recursion completes.
        let mut unconstrained = connection();
        unconstrained.batch_execute(setup).unwrap();
        assert!(
            crate::sql_query("INSERT INTO recur VALUES (1)")
                .execute(&mut unconstrained)
                .is_ok()
        );

        // A tiny depth limit is hit before the recursion can terminate
        // ("too many levels of trigger recursion").
        let mut conn = connection();
        conn.set_limit(SqliteLimit::TriggerDepth, 3);
        conn.batch_execute(setup).unwrap();
        assert!(
            crate::sql_query("INSERT INTO recur VALUES (1)")
                .execute(&mut conn)
                .is_err()
        );
    }

    #[diesel_test_helper::test]
    fn worker_threads_limit_has_no_runtime_error_path() {
        // Unlike the other categories, WorkerThreads only caps the number of auxiliary sort
        // threads a statement may start. Lowering it never raises an error, it only affects
        // performance. There is therefore no enforcement failure to assert, only that the value
        // is applied and ordinary queries keep working.
        let mut conn = connection();
        conn.set_limit(SqliteLimit::WorkerThreads, 0);
        assert_eq!(conn.get_limit(SqliteLimit::WorkerThreads), 0);
        assert!(crate::sql_query("SELECT 1").execute(&mut conn).is_ok());
    }

    #[diesel_test_helper::test]
    fn set_limit_enforces_sql_length() {
        let mut conn = connection();
        conn.set_limit(SqliteLimit::SqlLength, 20);

        // A statement longer than 20 bytes is rejected by SQLite.
        let result =
            crate::sql_query("SELECT * FROM sqlite_master WHERE type = 'table'").execute(&mut conn);
        assert!(result.is_err());
    }

    #[diesel_test_helper::test]
    fn set_limit_enforces_like_pattern_length() {
        let mut conn = connection();
        conn.set_limit(SqliteLimit::LikePatternLength, 100);

        assert!(
            crate::sql_query("SELECT 'test' LIKE 'te%'")
                .execute(&mut conn)
                .is_ok()
        );

        let long_pattern = "%".repeat(200);
        let query = format!("SELECT 'test' LIKE '{long_pattern}'");
        assert!(crate::sql_query(&query).execute(&mut conn).is_err());
    }

    #[diesel_test_helper::test]
    fn set_limit_clamps_above_compile_time_maximum() {
        let mut conn = connection();
        // SQLite clamps a requested value to its hard compile-time ceiling
        // rather than accepting it verbatim.
        conn.set_limit(SqliteLimit::Length, i32::MAX);
        let clamped = conn.get_limit(SqliteLimit::Length);
        assert!(clamped > 0 && clamped < i32::MAX);
    }

    #[diesel_test_helper::test]
    fn set_recommended_security_limits_applies_documented_table() {
        let mut conn = connection();
        conn.set_recommended_security_limits();

        assert_eq!(conn.get_limit(SqliteLimit::Length), 1_000_000);
        assert_eq!(conn.get_limit(SqliteLimit::SqlLength), 100_000);
        assert_eq!(conn.get_limit(SqliteLimit::ColumnCount), 100);
        assert_eq!(conn.get_limit(SqliteLimit::ExprDepth), 10);
        assert_eq!(conn.get_limit(SqliteLimit::CompoundSelect), 3);
        assert_eq!(conn.get_limit(SqliteLimit::VdbeOp), 25_000);
        assert_eq!(conn.get_limit(SqliteLimit::FunctionArg), 8);
        assert_eq!(conn.get_limit(SqliteLimit::Attached), 0);
        assert_eq!(conn.get_limit(SqliteLimit::LikePatternLength), 50);
        assert_eq!(conn.get_limit(SqliteLimit::VariableNumber), 10);
        assert_eq!(conn.get_limit(SqliteLimit::TriggerDepth), 10);
    }

    #[diesel_test_helper::test]
    fn safe_limit_constants_do_not_exceed_defaults() {
        // The hardened value for each category is a tightening of SQLite's published default, so
        // it must never be larger. This is asserted instead of comparing the `DEFAULT_*`
        // constants to a fresh connection, because the runtime default of categories such as
        // `FunctionArg` and `VariableNumber` is build-dependent (the bundled libsqlite3-sys
        // raises several of them), while these published constants are fixed.
        let pairs = [
            (
                SqliteLimit::SAFE_LENGTH_LIMIT,
                SqliteLimit::DEFAULT_LENGTH_LIMIT,
            ),
            (
                SqliteLimit::SAFE_SQL_LENGTH_LIMIT,
                SqliteLimit::DEFAULT_SQL_LENGTH_LIMIT,
            ),
            (
                SqliteLimit::SAFE_COLUMN_COUNT_LIMIT,
                SqliteLimit::DEFAULT_COLUMN_COUNT_LIMIT,
            ),
            (
                SqliteLimit::SAFE_EXPR_DEPTH_LIMIT,
                SqliteLimit::DEFAULT_EXPR_DEPTH_LIMIT,
            ),
            (
                SqliteLimit::SAFE_COMPOUND_SELECT_LIMIT,
                SqliteLimit::DEFAULT_COMPOUND_SELECT_LIMIT,
            ),
            (
                SqliteLimit::SAFE_VDBE_OP_LIMIT,
                SqliteLimit::DEFAULT_VDBE_OP_LIMIT,
            ),
            (
                SqliteLimit::SAFE_FUNCTION_ARG_LIMIT,
                SqliteLimit::DEFAULT_FUNCTION_ARG_LIMIT,
            ),
            (
                SqliteLimit::SAFE_ATTACHED_LIMIT,
                SqliteLimit::DEFAULT_ATTACHED_LIMIT,
            ),
            (
                SqliteLimit::SAFE_LIKE_PATTERN_LENGTH_LIMIT,
                SqliteLimit::DEFAULT_LIKE_PATTERN_LENGTH_LIMIT,
            ),
            (
                SqliteLimit::SAFE_VARIABLE_NUMBER_LIMIT,
                SqliteLimit::DEFAULT_VARIABLE_NUMBER_LIMIT,
            ),
            (
                SqliteLimit::SAFE_TRIGGER_DEPTH_LIMIT,
                SqliteLimit::DEFAULT_TRIGGER_DEPTH_LIMIT,
            ),
            (
                SqliteLimit::SAFE_WORKER_THREADS_LIMIT,
                SqliteLimit::DEFAULT_WORKER_THREADS_LIMIT,
            ),
        ];
        for (safe, default) in pairs {
            assert!(
                safe <= default,
                "safe value {safe} exceeds default {default}"
            );
        }
    }

    #[diesel_test_helper::test]
    fn safe_limit_constants_match_recommended_setter() {
        let mut conn = connection();
        conn.set_recommended_security_limits();

        assert_eq!(
            conn.get_limit(SqliteLimit::Length),
            SqliteLimit::SAFE_LENGTH_LIMIT
        );
        assert_eq!(
            conn.get_limit(SqliteLimit::SqlLength),
            SqliteLimit::SAFE_SQL_LENGTH_LIMIT
        );
        assert_eq!(
            conn.get_limit(SqliteLimit::ColumnCount),
            SqliteLimit::SAFE_COLUMN_COUNT_LIMIT
        );
        assert_eq!(
            conn.get_limit(SqliteLimit::ExprDepth),
            SqliteLimit::SAFE_EXPR_DEPTH_LIMIT
        );
        assert_eq!(
            conn.get_limit(SqliteLimit::CompoundSelect),
            SqliteLimit::SAFE_COMPOUND_SELECT_LIMIT
        );
        assert_eq!(
            conn.get_limit(SqliteLimit::VdbeOp),
            SqliteLimit::SAFE_VDBE_OP_LIMIT
        );
        assert_eq!(
            conn.get_limit(SqliteLimit::FunctionArg),
            SqliteLimit::SAFE_FUNCTION_ARG_LIMIT
        );
        assert_eq!(
            conn.get_limit(SqliteLimit::Attached),
            SqliteLimit::SAFE_ATTACHED_LIMIT
        );
        assert_eq!(
            conn.get_limit(SqliteLimit::LikePatternLength),
            SqliteLimit::SAFE_LIKE_PATTERN_LENGTH_LIMIT
        );
        assert_eq!(
            conn.get_limit(SqliteLimit::VariableNumber),
            SqliteLimit::SAFE_VARIABLE_NUMBER_LIMIT
        );
        assert_eq!(
            conn.get_limit(SqliteLimit::TriggerDepth),
            SqliteLimit::SAFE_TRIGGER_DEPTH_LIMIT
        );
        // The recommended setter leaves `WorkerThreads` untouched because its default is already
        // safe, so assert that the documented safe value matches what the connection reports.
        assert_eq!(
            conn.get_limit(SqliteLimit::WorkerThreads),
            SqliteLimit::SAFE_WORKER_THREADS_LIMIT
        );
    }

    // ---- db_config tests ----

    #[diesel_test_helper::test]
    fn db_config_defensive_roundtrip() {
        let conn = &mut connection();
        conn.set_defensive(true).unwrap();
        assert!(conn.is_defensive().unwrap());
        conn.set_defensive(false).unwrap();
        assert!(!conn.is_defensive().unwrap());
    }

    #[diesel_test_helper::test]
    fn db_config_trusted_schema_roundtrip() {
        let conn = &mut connection();
        conn.set_trusted_schema(false).unwrap();
        assert!(!conn.is_trusted_schema().unwrap());
        conn.set_trusted_schema(true).unwrap();
        assert!(conn.is_trusted_schema().unwrap());
    }

    #[diesel_test_helper::test]
    fn db_config_with_load_extension_enabled_scopes_the_flag() {
        let conn = &mut connection();
        conn.with_load_extension_enabled(|conn| {
            // Enabled for the duration of the closure.
            assert!(conn.is_load_extension_enabled().unwrap());
            QueryResult::Ok(())
        })
        .unwrap();
        // Disabled again afterwards.
        assert!(!conn.is_load_extension_enabled().unwrap());
    }

    #[cfg(all(
        feature = "std",
        not(all(target_family = "wasm", target_os = "unknown"))
    ))]
    #[diesel_test_helper::test]
    fn with_load_extension_enabled_disables_after_panic() {
        let conn = &mut connection();
        let outcome = std::panic::catch_unwind(core::panic::AssertUnwindSafe(|| {
            conn.with_load_extension_enabled(|_conn| -> QueryResult<()> {
                panic!("boom inside closure");
            })
        }));
        assert!(outcome.is_err(), "panic should propagate");
        assert!(
            !conn.is_load_extension_enabled().unwrap(),
            "extension loading must be disabled again after a panic"
        );
    }

    #[diesel_test_helper::test]
    fn db_config_triggers_roundtrip() {
        let conn = &mut connection();
        conn.set_triggers_enabled(false).unwrap();
        assert!(!conn.are_triggers_enabled().unwrap());
        conn.set_triggers_enabled(true).unwrap();
        assert!(conn.are_triggers_enabled().unwrap());
    }

    #[diesel_test_helper::test]
    fn db_config_views_roundtrip() {
        let conn = &mut connection();
        conn.set_views_enabled(false).unwrap();
        assert!(!conn.are_views_enabled().unwrap());
        conn.set_views_enabled(true).unwrap();
        assert!(conn.are_views_enabled().unwrap());
    }

    #[diesel_test_helper::test]
    fn db_config_foreign_keys_roundtrip() {
        let conn = &mut connection();
        conn.set_foreign_keys_enabled(true).unwrap();
        assert!(conn.are_foreign_keys_enabled().unwrap());
        conn.set_foreign_keys_enabled(false).unwrap();
        assert!(!conn.are_foreign_keys_enabled().unwrap());
    }

    #[diesel_test_helper::test]
    fn db_config_dqs_dml_roundtrip() {
        let conn = &mut connection();
        conn.set_double_quoted_strings_dml(false).unwrap();
        assert!(!conn.are_double_quoted_strings_dml_enabled().unwrap());
        conn.set_double_quoted_strings_dml(true).unwrap();
        assert!(conn.are_double_quoted_strings_dml_enabled().unwrap());
    }

    #[diesel_test_helper::test]
    fn db_config_dqs_ddl_roundtrip() {
        let conn = &mut connection();
        conn.set_double_quoted_strings_ddl(false).unwrap();
        assert!(!conn.are_double_quoted_strings_ddl_enabled().unwrap());
        conn.set_double_quoted_strings_ddl(true).unwrap();
        assert!(conn.are_double_quoted_strings_ddl_enabled().unwrap());
    }

    #[diesel_test_helper::test]
    fn db_config_fts3_tokenizer_roundtrip() {
        let conn = &mut connection();
        conn.set_fts3_tokenizer_enabled(false).unwrap();
        assert!(!conn.is_fts3_tokenizer_enabled().unwrap());
        conn.set_fts3_tokenizer_enabled(true).unwrap();
        assert!(conn.is_fts3_tokenizer_enabled().unwrap());
    }

    #[diesel_test_helper::test]
    fn db_config_writable_schema_roundtrip() {
        let conn = &mut connection();
        conn.set_writable_schema(false).unwrap();
        assert!(!conn.is_writable_schema().unwrap());
        conn.set_writable_schema(true).unwrap();
        assert!(conn.is_writable_schema().unwrap());
    }

    #[diesel_test_helper::test]
    fn db_config_attach_create_roundtrip() {
        let conn = &mut connection();
        // ATTACH_CREATE requires SQLite 3.46.0+; skip if unsupported
        if conn.set_attach_create_enabled(false).is_err() {
            return;
        }
        assert!(!conn.is_attach_create_enabled().unwrap());
        conn.set_attach_create_enabled(true).unwrap();
        assert!(conn.is_attach_create_enabled().unwrap());
    }

    #[diesel_test_helper::test]
    fn db_config_attach_write_roundtrip() {
        let conn = &mut connection();
        // ATTACH_WRITE requires SQLite 3.46.0+; skip if unsupported
        if conn.set_attach_write_enabled(false).is_err() {
            return;
        }
        assert!(!conn.is_attach_write_enabled().unwrap());
        conn.set_attach_write_enabled(true).unwrap();
        assert!(conn.is_attach_write_enabled().unwrap());
    }

    // ---- behavioral db_config tests ----

    #[diesel_test_helper::test]
    fn defensive_mode_blocks_writable_schema() {
        let conn = &mut connection();
        conn.set_defensive(true).unwrap();
        // In defensive mode, writable_schema should remain off even if we try to set it
        let _ = crate::sql_query("PRAGMA writable_schema = ON").execute(conn);
        assert!(!conn.is_writable_schema().unwrap());
    }

    #[diesel_test_helper::test]
    fn foreign_keys_enabled_enforces_constraints() {
        let conn = &mut connection();
        conn.set_foreign_keys_enabled(true).unwrap();

        crate::sql_query("CREATE TABLE parent (id INTEGER PRIMARY KEY)")
            .execute(conn)
            .unwrap();
        crate::sql_query(
            "CREATE TABLE child (id INTEGER PRIMARY KEY, parent_id INTEGER REFERENCES parent(id))",
        )
        .execute(conn)
        .unwrap();

        // Insert a child row with no matching parent — should fail with FK enabled
        let result =
            crate::sql_query("INSERT INTO child (id, parent_id) VALUES (1, 999)").execute(conn);
        assert!(result.is_err());
    }

    #[diesel_test_helper::test]
    fn views_disabled_blocks_view_queries() {
        let conn = &mut connection();
        crate::sql_query("CREATE TABLE base (id INTEGER PRIMARY KEY)")
            .execute(conn)
            .unwrap();
        crate::sql_query("INSERT INTO base (id) VALUES (1)")
            .execute(conn)
            .unwrap();
        crate::sql_query("CREATE VIEW base_view AS SELECT id FROM base")
            .execute(conn)
            .unwrap();

        // Enabled (default): the view can be queried.
        conn.set_views_enabled(true).unwrap();
        assert!(
            crate::sql_query("SELECT id FROM base_view")
                .execute(conn)
                .is_ok()
        );

        // Disabled: queries that reference the view fail.
        conn.set_views_enabled(false).unwrap();
        assert!(
            crate::sql_query("SELECT id FROM base_view")
                .execute(conn)
                .is_err()
        );
    }

    #[diesel_test_helper::test]
    fn triggers_disabled_prevents_firing() {
        let conn = &mut connection();
        crate::sql_query("CREATE TABLE source (id INTEGER PRIMARY KEY)")
            .execute(conn)
            .unwrap();
        crate::sql_query("CREATE TABLE trigger_log (n INTEGER)")
            .execute(conn)
            .unwrap();
        crate::sql_query("CREATE TRIGGER log_insert AFTER INSERT ON source BEGIN INSERT INTO trigger_log (n) VALUES (1); END")
            .execute(conn)
            .unwrap();

        // Disabled: inserting into `source` must not fire the trigger.
        conn.set_triggers_enabled(false).unwrap();
        crate::sql_query("INSERT INTO source (id) VALUES (1)")
            .execute(conn)
            .unwrap();
        let count: i64 = sql::<crate::sql_types::BigInt>("SELECT COUNT(*) FROM trigger_log")
            .get_result(conn)
            .unwrap();
        assert_eq!(0, count, "trigger should not fire while disabled");

        // Enabled: the trigger fires and writes one row.
        conn.set_triggers_enabled(true).unwrap();
        crate::sql_query("INSERT INTO source (id) VALUES (2)")
            .execute(conn)
            .unwrap();
        let count: i64 = sql::<crate::sql_types::BigInt>("SELECT COUNT(*) FROM trigger_log")
            .get_result(conn)
            .unwrap();
        assert_eq!(1, count, "trigger should fire while enabled");
    }

    #[diesel_test_helper::test]
    fn dqs_dml_controls_double_quoted_string_literals() {
        let conn = &mut connection();

        // Disabled: a double-quoted token in DML is parsed as an identifier, so a
        // bare `"text"` that is not a column errors.
        conn.set_double_quoted_strings_dml(false).unwrap();
        let disabled = sql::<Text>(r#"SELECT "bare_token""#).get_result::<String>(conn);
        assert!(disabled.is_err());

        // Enabled: the same token is accepted as a string literal.
        conn.set_double_quoted_strings_dml(true).unwrap();
        let enabled = sql::<Text>(r#"SELECT "bare_token""#).get_result::<String>(conn);
        assert_eq!(Ok("bare_token".to_owned()), enabled);
    }

    #[diesel_test_helper::test]
    fn dqs_ddl_controls_double_quoted_string_literals() {
        let conn = &mut connection();

        // Disabled: a double-quoted token in a CHECK constraint is parsed as an
        // identifier. As there is no such column, creating the table errors.
        conn.set_double_quoted_strings_ddl(false).unwrap();
        let disabled =
            crate::sql_query(r#"CREATE TABLE dqs_off (name TEXT, CHECK (name <> "not_a_column"))"#)
                .execute(conn);
        assert!(disabled.is_err());

        // Enabled: the same token is accepted as a string literal, so the CHECK
        // constraint (and the table) are created successfully.
        conn.set_double_quoted_strings_ddl(true).unwrap();
        let enabled =
            crate::sql_query(r#"CREATE TABLE dqs_on (name TEXT, CHECK (name <> "not_a_column"))"#)
                .execute(conn);
        assert!(enabled.is_ok());
    }

    #[diesel_test_helper::test]
    fn writable_schema_controls_direct_sqlite_master_writes() {
        let conn = &mut connection();
        crate::sql_query("CREATE TABLE protected (id INTEGER PRIMARY KEY)")
            .execute(conn)
            .unwrap();

        let update =
            "UPDATE sqlite_master SET sql = sql WHERE type = 'table' AND name = 'protected'";

        // Disabled (default): a direct write to sqlite_master is rejected.
        conn.set_writable_schema(false).unwrap();
        assert!(crate::sql_query(update).execute(conn).is_err());

        // Enabled: the same write is permitted.
        conn.set_writable_schema(true).unwrap();
        assert!(crate::sql_query(update).execute(conn).is_ok());
    }

    #[diesel_test_helper::test]
    fn fts3_tokenizer_disabled_blocks_the_function() {
        let conn = &mut connection();

        // Enable first to detect whether FTS3 is compiled into this SQLite build.
        conn.set_fts3_tokenizer_enabled(true).unwrap();
        let enabled = sql::<crate::sql_types::Binary>("SELECT fts3_tokenizer('simple')")
            .get_result::<Vec<u8>>(conn);
        if enabled.is_err() {
            // FTS3 is not available in this build, so there is nothing to assert.
            return;
        }

        // Disabled: the `fts3_tokenizer()` SQL function is no longer callable.
        conn.set_fts3_tokenizer_enabled(false).unwrap();
        let disabled = sql::<crate::sql_types::Binary>("SELECT fts3_tokenizer('simple')")
            .get_result::<Vec<u8>>(conn);
        assert!(disabled.is_err());
    }

    // These ATTACH tests need a real filesystem (temp files), which is not
    // available on the wasm target, where SQLite is in-memory only.
    #[cfg(not(all(target_family = "wasm", target_os = "unknown")))]
    fn temp_db_path(tag: &str) -> std::path::PathBuf {
        let mut path = std::env::temp_dir();
        path.push(format!("diesel_attach_{}_{}.db", std::process::id(), tag));
        path
    }

    #[cfg(not(all(target_family = "wasm", target_os = "unknown")))]
    #[diesel_test_helper::test]
    fn attach_create_disabled_blocks_new_database_files() {
        let conn = &mut connection();

        // The ATTACH_CREATE option was added in SQLite 3.49.0; skip on older
        // libraries (e.g. the system SQLite on the Ubuntu 24.04 CI runners).
        if conn.set_attach_create_enabled(false).is_err() {
            return;
        }

        let path = temp_db_path("create");
        let _ = std::fs::remove_file(&path);
        let attach = format!("ATTACH DATABASE '{}' AS aux_create", path.display());

        // Disabled: attaching a path that does not exist yet must fail.
        assert!(crate::sql_query(&attach).execute(conn).is_err());

        // Enabled: the same ATTACH now creates and opens the file.
        conn.set_attach_create_enabled(true).unwrap();
        crate::sql_query(&attach).execute(conn).unwrap();
        crate::sql_query("DETACH DATABASE aux_create")
            .execute(conn)
            .unwrap();

        let _ = std::fs::remove_file(&path);
    }

    #[cfg(not(all(target_family = "wasm", target_os = "unknown")))]
    #[diesel_test_helper::test]
    fn attach_write_disabled_opens_attached_databases_read_only() {
        let conn = &mut connection();

        // The ATTACH_WRITE option was added in SQLite 3.49.0; skip on older
        // libraries (e.g. the system SQLite on the Ubuntu 24.04 CI runners).
        // This guard also leaves ATTACH_WRITE disabled for the first check below.
        if conn.set_attach_write_enabled(false).is_err() {
            return;
        }

        // Seed an existing on-disk database with a table to write into.
        let path = temp_db_path("write");
        let _ = std::fs::remove_file(&path);
        {
            let mut seed = SqliteConnection::establish(path.to_str().unwrap()).unwrap();
            crate::sql_query("CREATE TABLE t (id INTEGER)")
                .execute(&mut seed)
                .unwrap();
        }
        let attach = format!("ATTACH DATABASE '{}' AS aux_write", path.display());

        // Disabled: the attached database is opened read-only, so writes fail.
        crate::sql_query(&attach).execute(conn).unwrap();
        assert!(
            crate::sql_query("INSERT INTO aux_write.t (id) VALUES (1)")
                .execute(conn)
                .is_err()
        );
        crate::sql_query("DETACH DATABASE aux_write")
            .execute(conn)
            .unwrap();

        // Enabled: the attached database is writable again.
        conn.set_attach_write_enabled(true).unwrap();
        crate::sql_query(&attach).execute(conn).unwrap();
        crate::sql_query("INSERT INTO aux_write.t (id) VALUES (1)")
            .execute(conn)
            .unwrap();
        crate::sql_query("DETACH DATABASE aux_write")
            .execute(conn)
            .unwrap();

        let _ = std::fs::remove_file(&path);
    }

    // ---- DIRECTONLY / INNOCUOUS function behavior tests ----

    #[declare_sql_function]
    extern "SQL" {
        fn directonly_fn() -> Integer;
        fn innocuous_fn() -> Integer;
    }

    #[diesel_test_helper::test]
    fn directonly_function_blocked_from_view() {
        let conn = &mut connection();

        // Register a DIRECTONLY function
        directonly_fn_utils::register_impl_with_behavior(
            conn,
            SqliteFunctionBehavior::DIRECTONLY,
            || 42,
        )
        .unwrap();

        // Direct call works
        let result = crate::select(directonly_fn()).get_result::<i32>(conn);
        assert_eq!(Ok(42), result);

        // Create a view that calls the function
        crate::sql_query("CREATE VIEW test_view AS SELECT directonly_fn() AS val")
            .execute(conn)
            .unwrap();

        // Disable trusted schema so DIRECTONLY is enforced from schema objects
        conn.set_trusted_schema(false).unwrap();

        // Querying the view should fail because the function is DIRECTONLY
        let result = crate::sql_query("SELECT val FROM test_view").execute(conn);
        assert!(result.is_err());
    }

    #[diesel_test_helper::test]
    fn innocuous_function_allowed_from_view_with_untrusted_schema() {
        let conn = &mut connection();

        // Register an INNOCUOUS function
        innocuous_fn_utils::register_impl_with_behavior(
            conn,
            SqliteFunctionBehavior::DETERMINISTIC | SqliteFunctionBehavior::INNOCUOUS,
            || 99,
        )
        .unwrap();

        // Create a view that calls the function
        crate::sql_query("CREATE VIEW innocuous_view AS SELECT innocuous_fn() AS val")
            .execute(conn)
            .unwrap();

        // Disable trusted schema
        conn.set_trusted_schema(false).unwrap();

        // Querying the view should succeed because the function is INNOCUOUS
        let result = crate::sql_query("SELECT val FROM innocuous_view").execute(conn);
        assert!(result.is_ok());
    }
}
