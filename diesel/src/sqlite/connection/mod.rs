#[cfg(not(all(target_family = "wasm", target_os = "unknown")))]
extern crate libsqlite3_sys as ffi;

#[cfg(all(target_family = "wasm", target_os = "unknown"))]
use sqlite_wasm_rs as ffi;

// Option codes for `sqlite3_db_config()` that control whether the ATTACH
// statement is allowed to create new database files (ATTACH_CREATE) or open
// them in write mode (ATTACH_WRITE).  They are passed to
// `set_db_config_bool` / `get_db_config_bool` in the public
// `set_attach_create_enabled` and `set_attach_write_enabled` methods.
//
// These constants were introduced in SQLite 3.49.0 and are only present in
// `libsqlite3-sys` >= 0.35.0.  Diesel supports `libsqlite3-sys` >= 0.17.2,
// so the constants may be absent at compile time.  We define them here so
// that diesel compiles against any supported `libsqlite3-sys` version; if
// the linked SQLite library is too old to recognise the option, the
// `sqlite3_db_config()` call will return an error at runtime, which the
// calling code already handles.
const SQLITE_DBCONFIG_ENABLE_ATTACH_CREATE: i32 = 1020;
const SQLITE_DBCONFIG_ENABLE_ATTACH_WRITE: i32 = 1021;

mod bind_collector;
mod functions;
mod owned_row;
mod raw;
mod row;
mod serialized_database;
mod sqlite_value;
mod statement_iterator;
mod stmt;

pub(in crate::sqlite) use self::bind_collector::SqliteBindCollector;
pub use self::bind_collector::SqliteBindValue;
pub use self::serialized_database::SerializedDatabase;
pub use self::sqlite_value::SqliteValue;

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
use core::ffi as libc;

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
    pub fn deserialize_readonly_database_from_buffer(&mut self, data: &[u8]) -> QueryResult<()> {
        self.raw_connection.deserialize(data)
    }

    /// Enable or disable SQLite defensive mode.
    ///
    /// When enabled, defensive mode prevents:
    /// - Direct writes to shadow tables (used by FTS5, R-Tree, etc.)
    /// - Dangerous PRAGMA commands like `writable_schema`
    /// - `sqlite3_deserialize()` from opening unsafe database images
    /// - Other potentially dangerous operations
    ///
    /// # Security Hardening Recipe
    ///
    /// For recommended security hardening of SQLite connections:
    ///
    /// ```rust
    /// # include!("../../doctest_setup.rs");
    /// # fn main() {
    /// #     let mut conn = SqliteConnection::establish(":memory:").unwrap();
    /// conn.set_defensive(true).unwrap();
    /// conn.set_trusted_schema(false).unwrap();
    /// conn.set_load_extension_enabled(false).unwrap();
    /// # }
    /// ```
    ///
    /// # Availability
    ///
    /// Requires SQLite 3.26.0 (2018-12) or later. Returns an error if the
    /// linked SQLite version does not support this option.
    ///
    /// # Security Recommendation
    ///
    /// Enable defensive mode for any connection that may process untrusted data.
    /// This is the single most important security flag for hardening SQLite.
    ///
    /// See [SQLite documentation](https://sqlite.org/c3ref/db_config.html) for more details.
    ///
    /// # Example
    ///
    /// ```rust
    /// # include!("../../doctest_setup.rs");
    /// # fn main() {
    /// #     let mut conn = SqliteConnection::establish(":memory:").unwrap();
    /// // Enable defensive mode for security hardening
    /// conn.set_defensive(true).unwrap();
    /// assert!(conn.is_defensive().unwrap());
    /// # }
    /// ```
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
    /// columns, expression indexes) are restricted to only those marked
    /// with [`SqliteFunctionBehavior::INNOCUOUS`][crate::sqlite::SqliteFunctionBehavior::INNOCUOUS].
    ///
    /// # Availability
    ///
    /// Requires SQLite 3.31.0 (2020-01) or later. Returns an error if the
    /// linked SQLite version does not support this option.
    ///
    /// # Security Recommendation
    ///
    /// Disable trusted schema when opening database files from untrusted sources.
    /// When disabled, ensure your custom SQL functions are registered with
    /// appropriate behavior flags (see [`SqliteFunctionBehavior`][crate::sqlite::SqliteFunctionBehavior]).
    ///
    /// See [SQLite documentation](https://sqlite.org/c3ref/db_config.html) for more details.
    ///
    /// # Example
    ///
    /// ```rust
    /// # include!("../../doctest_setup.rs");
    /// # fn main() {
    /// #     let mut conn = SqliteConnection::establish(":memory:").unwrap();
    /// // Disable trusted schema for security hardening
    /// conn.set_trusted_schema(false).unwrap();
    /// assert!(!conn.is_trusted_schema().unwrap());
    /// # }
    /// ```
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

    /// Enable or disable the `load_extension()` SQL function.
    ///
    /// This controls whether the [`load_extension()`](https://www.sqlite.org/lang_corefunc.html#load_extension)
    /// **SQL function** can be called. It does **not** affect the
    /// `sqlite3_load_extension()` C API (which Diesel does not expose).
    /// The C API is controlled separately by `sqlite3_enable_load_extension()`.
    ///
    /// # Availability
    ///
    /// Requires SQLite 3.13.0 (2016-05) or later. Returns an error if the
    /// linked SQLite version does not support this option.
    ///
    /// If SQLite was compiled with `SQLITE_OMIT_LOAD_EXTENSION`, the
    /// `load_extension()` SQL function does not exist and this setting
    /// has no effect.
    ///
    /// # Security Recommendation
    ///
    /// Disable extension loading unless specifically needed.
    ///
    /// See [SQLite documentation](https://sqlite.org/c3ref/db_config.html) for more details.
    ///
    /// # Example
    ///
    /// ```rust
    /// # include!("../../doctest_setup.rs");
    /// # fn main() {
    /// #     let mut conn = SqliteConnection::establish(":memory:").unwrap();
    /// // Disable extension loading for security
    /// conn.set_load_extension_enabled(false).unwrap();
    /// assert!(!conn.is_load_extension_enabled().unwrap());
    /// # }
    /// ```
    pub fn set_load_extension_enabled(&mut self, enabled: bool) -> QueryResult<()> {
        self.raw_connection
            .set_db_config_bool(ffi::SQLITE_DBCONFIG_ENABLE_LOAD_EXTENSION, enabled)
    }

    /// Check if `load_extension()` SQL function is enabled.
    ///
    /// See [`set_load_extension_enabled`][Self::set_load_extension_enabled] for details.
    pub fn is_load_extension_enabled(&self) -> QueryResult<bool> {
        self.raw_connection
            .get_db_config_bool(ffi::SQLITE_DBCONFIG_ENABLE_LOAD_EXTENSION)
    }

    /// Enable or disable the `fts3_tokenizer()` SQL function.
    ///
    /// When enabled, the [`fts3_tokenizer()`](https://www.sqlite.org/fts3.html#f3tknzr)
    /// function allows overloading the default FTS3/FTS4 tokenizer, which
    /// can be exploited if an attacker can execute arbitrary SQL.
    ///
    /// # Availability
    ///
    /// Requires SQLite 3.12.0 (2016-03) or later. Returns an error if the
    /// linked SQLite version does not support this option.
    ///
    /// # Security Recommendation
    ///
    /// Disable this unless you specifically need custom FTS3 tokenizers.
    ///
    /// See [SQLite documentation](https://sqlite.org/c3ref/db_config.html) for more details.
    ///
    /// # Example
    ///
    /// ```rust
    /// # include!("../../doctest_setup.rs");
    /// # fn main() {
    /// #     let mut conn = SqliteConnection::establish(":memory:").unwrap();
    /// // Disable FTS3 tokenizer overloading for security
    /// conn.set_fts3_tokenizer_enabled(false).unwrap();
    /// assert!(!conn.is_fts3_tokenizer_enabled().unwrap());
    /// # }
    /// ```
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
    /// When enabled, allows direct modification of the `sqlite_master`
    /// table, which can corrupt the database if misused.
    ///
    /// # Availability
    ///
    /// Requires SQLite 3.28.0 (2019-04) or later. Returns an error if the
    /// linked SQLite version does not support this option.
    ///
    /// # Security Recommendation
    ///
    /// Keep this disabled unless you specifically need to repair or modify
    /// the database schema directly. Enabling defensive mode
    /// ([`set_defensive`][Self::set_defensive]) will also prevent this.
    ///
    /// See [SQLite documentation](https://sqlite.org/c3ref/db_config.html) for more details.
    ///
    /// # Example
    ///
    /// ```rust
    /// # include!("../../doctest_setup.rs");
    /// # fn main() {
    /// #     let mut conn = SqliteConnection::establish(":memory:").unwrap();
    /// // Ensure writable schema is disabled for safety
    /// conn.set_writable_schema(false).unwrap();
    /// assert!(!conn.is_writable_schema().unwrap());
    /// # }
    /// ```
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
    /// When disabled, [`ATTACH`](https://www.sqlite.org/lang_attach.html)
    /// can only open existing database files; it cannot create new ones.
    ///
    /// # Availability
    ///
    /// Requires SQLite 3.49.0 (2025-02) or later. Returns an error if the
    /// linked SQLite version does not support this option.
    ///
    /// # Security Recommendation
    ///
    /// Disable this in environments where database file creation should
    /// be restricted.
    ///
    /// See [SQLite documentation](https://sqlite.org/c3ref/db_config.html) for more details.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # include!("../../doctest_setup.rs");
    /// # fn main() {
    /// #     let mut conn = SqliteConnection::establish(":memory:").unwrap();
    /// // Disable ATTACH file creation for security
    /// conn.set_attach_create_enabled(false).unwrap();
    /// assert!(!conn.is_attach_create_enabled().unwrap());
    /// # }
    /// ```
    pub fn set_attach_create_enabled(&mut self, enabled: bool) -> QueryResult<()> {
        self.raw_connection
            .set_db_config_bool(SQLITE_DBCONFIG_ENABLE_ATTACH_CREATE, enabled)
    }

    /// Check if ATTACH can create new database files.
    ///
    /// See [`set_attach_create_enabled`][Self::set_attach_create_enabled] for details.
    pub fn is_attach_create_enabled(&self) -> QueryResult<bool> {
        self.raw_connection
            .get_db_config_bool(SQLITE_DBCONFIG_ENABLE_ATTACH_CREATE)
    }

    /// Enable or disable ATTACH from opening databases in write mode.
    ///
    /// When disabled, all attached databases are opened as read-only.
    ///
    /// # Availability
    ///
    /// Requires SQLite 3.49.0 (2025-02) or later. Returns an error if the
    /// linked SQLite version does not support this option.
    ///
    /// # Security Recommendation
    ///
    /// Disable this to restrict write access to attached databases.
    ///
    /// See [SQLite documentation](https://sqlite.org/c3ref/db_config.html) for more details.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # include!("../../doctest_setup.rs");
    /// # fn main() {
    /// #     let mut conn = SqliteConnection::establish(":memory:").unwrap();
    /// // Disable ATTACH write access for security
    /// conn.set_attach_write_enabled(false).unwrap();
    /// assert!(!conn.is_attach_write_enabled().unwrap());
    /// # }
    /// ```
    pub fn set_attach_write_enabled(&mut self, enabled: bool) -> QueryResult<()> {
        self.raw_connection
            .set_db_config_bool(SQLITE_DBCONFIG_ENABLE_ATTACH_WRITE, enabled)
    }

    /// Check if ATTACH can open databases in write mode.
    ///
    /// See [`set_attach_write_enabled`][Self::set_attach_write_enabled] for details.
    pub fn is_attach_write_enabled(&self) -> QueryResult<bool> {
        self.raw_connection
            .get_db_config_bool(SQLITE_DBCONFIG_ENABLE_ATTACH_WRITE)
    }

    /// Enable or disable trigger execution.
    ///
    /// When disabled, triggers will not fire for any DML operations.
    ///
    /// # Availability
    ///
    /// Requires SQLite 3.8.7 (2014-10) or later. Returns an error if the
    /// linked SQLite version does not support this option.
    ///
    /// See [SQLite documentation](https://sqlite.org/c3ref/db_config.html) for more details.
    ///
    /// # Example
    ///
    /// ```rust
    /// # include!("../../doctest_setup.rs");
    /// # fn main() {
    /// #     let mut conn = SqliteConnection::establish(":memory:").unwrap();
    /// // Disable triggers temporarily
    /// conn.set_triggers_enabled(false).unwrap();
    /// assert!(!conn.are_triggers_enabled().unwrap());
    ///
    /// // Re-enable triggers
    /// conn.set_triggers_enabled(true).unwrap();
    /// # }
    /// ```
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
    /// # Availability
    ///
    /// Requires SQLite 3.30.0 (2019-10) or later. Returns an error if the
    /// linked SQLite version does not support this option.
    ///
    /// See [SQLite documentation](https://sqlite.org/c3ref/db_config.html) for more details.
    ///
    /// # Example
    ///
    /// ```rust
    /// # include!("../../doctest_setup.rs");
    /// # fn main() {
    /// #     let mut conn = SqliteConnection::establish(":memory:").unwrap();
    /// // Disable views temporarily
    /// conn.set_views_enabled(false).unwrap();
    /// assert!(!conn.are_views_enabled().unwrap());
    /// # }
    /// ```
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
    /// # Availability
    ///
    /// Requires SQLite 3.8.7 (2014-10) or later. Returns an error if the
    /// linked SQLite version does not support this option.
    ///
    /// See [SQLite documentation](https://sqlite.org/c3ref/db_config.html) for more details.
    ///
    /// # Example
    ///
    /// ```rust
    /// # include!("../../doctest_setup.rs");
    /// # fn main() {
    /// #     let mut conn = SqliteConnection::establish(":memory:").unwrap();
    /// // Enable foreign key enforcement
    /// conn.set_foreign_keys_enabled(true).unwrap();
    /// assert!(conn.are_foreign_keys_enabled().unwrap());
    /// # }
    /// ```
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
    /// rather than identifiers. This is a legacy behavior that can cause issues.
    ///
    /// # Availability
    ///
    /// Requires SQLite 3.29.0 (2019-07) or later. Returns an error if the
    /// linked SQLite version does not support this option.
    ///
    /// # Recommendation
    ///
    /// Disable this for stricter SQL compliance.
    ///
    /// See [SQLite documentation](https://sqlite.org/c3ref/db_config.html) for more details.
    ///
    /// # Example
    ///
    /// ```rust
    /// # include!("../../doctest_setup.rs");
    /// # fn main() {
    /// #     let mut conn = SqliteConnection::establish(":memory:").unwrap();
    /// // Disable double-quoted strings in DML for stricter SQL
    /// conn.set_double_quoted_string_dml(false).unwrap();
    /// # }
    /// ```
    pub fn set_double_quoted_string_dml(&mut self, enabled: bool) -> QueryResult<()> {
        self.raw_connection
            .set_db_config_bool(ffi::SQLITE_DBCONFIG_DQS_DML, enabled)
    }

    /// Check if double-quoted strings in DML are enabled.
    ///
    /// See [`set_double_quoted_string_dml`][Self::set_double_quoted_string_dml] for details.
    pub fn is_double_quoted_string_dml_enabled(&self) -> QueryResult<bool> {
        self.raw_connection
            .get_db_config_bool(ffi::SQLITE_DBCONFIG_DQS_DML)
    }

    /// Enable or disable double-quoted strings in DDL statements.
    ///
    /// When enabled, double-quoted strings are interpreted as string literals
    /// rather than identifiers. This is a legacy behavior that can cause issues.
    ///
    /// # Availability
    ///
    /// Requires SQLite 3.29.0 (2019-07) or later. Returns an error if the
    /// linked SQLite version does not support this option.
    ///
    /// # Recommendation
    ///
    /// Disable this for stricter SQL compliance.
    ///
    /// See [SQLite documentation](https://sqlite.org/c3ref/db_config.html) for more details.
    ///
    /// # Example
    ///
    /// ```rust
    /// # include!("../../doctest_setup.rs");
    /// # fn main() {
    /// #     let mut conn = SqliteConnection::establish(":memory:").unwrap();
    /// // Disable double-quoted strings in DDL for stricter SQL
    /// conn.set_double_quoted_string_ddl(false).unwrap();
    /// # }
    /// ```
    pub fn set_double_quoted_string_ddl(&mut self, enabled: bool) -> QueryResult<()> {
        self.raw_connection
            .set_db_config_bool(ffi::SQLITE_DBCONFIG_DQS_DDL, enabled)
    }

    /// Check if double-quoted strings in DDL are enabled.
    ///
    /// See [`set_double_quoted_string_ddl`][Self::set_double_quoted_string_ddl] for details.
    pub fn is_double_quoted_string_ddl_enabled(&self) -> QueryResult<bool> {
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
        };
        conn.register_diesel_sql_functions()
            .map_err(CouldntSetupConfiguration)?;
        Ok(conn)
    }
}

fn error_message(err_code: libc::c_int) -> &'static str {
    ffi::code_to_str(err_code)
}

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

        let serialized_database = conn1.serialize_database_to_buffer();

        let conn2 = &mut connection();
        conn2
            .deserialize_readonly_database_from_buffer(serialized_database.as_slice())
            .unwrap();

        let query = sql::<(Integer, Text, Text)>("SELECT id, name, email FROM users ORDER BY id");
        let actual_users = query.load::<(i32, String, String)>(conn2).unwrap();

        assert_eq!(expected_users, actual_users);
    }

    #[diesel_test_helper::test]
    fn register_custom_function() {
        let connection = &mut connection();
        fun_case_utils::register_impl(
            connection,
            SqliteFunctionBehavior::DETERMINISTIC,
            |x: String| {
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
            },
        )
        .unwrap();

        let mapped_string = crate::select(fun_case("foobar"))
            .get_result::<String>(connection)
            .unwrap();
        assert_eq!("fOoBaR", mapped_string);
    }

    #[diesel_test_helper::test]
    fn register_multiarg_function() {
        let connection = &mut connection();
        my_add_utils::register_impl(
            connection,
            SqliteFunctionBehavior::DETERMINISTIC,
            |x: i32, y: i32| x + y,
        )
        .unwrap();

        let added = crate::select(my_add(1, 2)).get_result::<i32>(connection);
        assert_eq!(Ok(3), added);
    }

    #[diesel_test_helper::test]
    fn register_noarg_function() {
        let connection = &mut connection();
        answer_utils::register_impl(connection, SqliteFunctionBehavior::DETERMINISTIC, || 42)
            .unwrap();

        let answer = crate::select(answer()).get_result::<i32>(connection);
        assert_eq!(Ok(42), answer);
    }

    #[diesel_test_helper::test]
    fn register_nondeterministic_noarg_function() {
        let connection = &mut connection();
        answer_utils::register_impl(connection, SqliteFunctionBehavior::empty(), || 42).unwrap();

        let answer = crate::select(answer()).get_result::<i32>(connection);
        assert_eq!(Ok(42), answer);
    }

    #[diesel_test_helper::test]
    fn register_nondeterministic_function() {
        let connection = &mut connection();
        let mut y = 0;
        add_counter_utils::register_impl(
            connection,
            SqliteFunctionBehavior::empty(),
            move |x: i32| {
                y += 1;
                x + y
            },
        )
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

        my_sum_utils::register_impl::<MySum, _>(connection, SqliteFunctionBehavior::DETERMINISTIC)
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

        my_sum_utils::register_impl::<MySum, _>(connection, SqliteFunctionBehavior::DETERMINISTIC)
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

        range_max_utils::register_impl::<RangeMax<i32>, _, _, _>(
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
    fn db_config_load_extension_roundtrip() {
        let conn = &mut connection();
        conn.set_load_extension_enabled(false).unwrap();
        assert!(!conn.is_load_extension_enabled().unwrap());
        conn.set_load_extension_enabled(true).unwrap();
        assert!(conn.is_load_extension_enabled().unwrap());
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
        conn.set_double_quoted_string_dml(false).unwrap();
        assert!(!conn.is_double_quoted_string_dml_enabled().unwrap());
        conn.set_double_quoted_string_dml(true).unwrap();
        assert!(conn.is_double_quoted_string_dml_enabled().unwrap());
    }

    #[diesel_test_helper::test]
    fn db_config_dqs_ddl_roundtrip() {
        let conn = &mut connection();
        conn.set_double_quoted_string_ddl(false).unwrap();
        assert!(!conn.is_double_quoted_string_ddl_enabled().unwrap());
        conn.set_double_quoted_string_ddl(true).unwrap();
        assert!(conn.is_double_quoted_string_ddl_enabled().unwrap());
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
        directonly_fn_utils::register_impl(conn, SqliteFunctionBehavior::DIRECTONLY, || 42)
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
        innocuous_fn_utils::register_impl(
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
