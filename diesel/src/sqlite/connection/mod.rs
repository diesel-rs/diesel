#[cfg(not(all(target_family = "wasm", target_os = "unknown")))]
extern crate libsqlite3_sys as ffi;

#[cfg(all(target_family = "wasm", target_os = "unknown"))]
use sqlite_wasm_rs as ffi;

mod bind_collector;
mod functions;
mod owned_row;
mod raw;
mod row;
mod serialized_database;
pub(in crate::sqlite) mod sqlite_blob;
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
use crate::sqlite::Sqlite;
use alloc::string::String;
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
        deterministic: bool,
        mut f: F,
    ) -> QueryResult<()>
    where
        F: FnMut(Args) -> Ret + core::panic::UnwindSafe + Send + 'static,
        Args: FromSqlRow<ArgsSqlType, Sqlite> + StaticallySizedRow<ArgsSqlType, Sqlite>,
        Ret: ToSql<RetSqlType, Sqlite>,
        Sqlite: HasSqlType<RetSqlType>,
    {
        functions::register(
            &self.raw_connection,
            fn_name,
            deterministic,
            move |_, args| f(args),
        )
    }

    #[doc(hidden)]
    pub fn register_noarg_sql_function<RetSqlType, Ret, F>(
        &self,
        fn_name: &str,
        deterministic: bool,
        f: F,
    ) -> QueryResult<()>
    where
        F: FnMut() -> Ret + core::panic::UnwindSafe + Send + 'static,
        Ret: ToSql<RetSqlType, Sqlite>,
        Sqlite: HasSqlType<RetSqlType>,
    {
        functions::register_noargs(&self.raw_connection, fn_name, deterministic, f)
    }

    #[doc(hidden)]
    pub fn register_aggregate_function<ArgsSqlType, RetSqlType, Args, Ret, A>(
        &mut self,
        fn_name: &str,
    ) -> QueryResult<()>
    where
        A: SqliteAggregateFunction<Args, Output = Ret> + 'static + Send + core::panic::UnwindSafe,
        Args: FromSqlRow<ArgsSqlType, Sqlite> + StaticallySizedRow<ArgsSqlType, Sqlite>,
        Ret: ToSql<RetSqlType, Sqlite>,
        Sqlite: HasSqlType<RetSqlType>,
    {
        functions::register_aggregate::<_, _, _, _, A>(&self.raw_connection, fn_name)
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

    fn register_diesel_sql_functions(&self) -> QueryResult<()> {
        use crate::sql_types::{Integer, Text};

        functions::register::<Text, Integer, _, _, _>(
            &self.raw_connection,
            "diesel_manage_updated_at",
            false,
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

        my_sum_utils::register_impl::<MySum, _>(connection).unwrap();

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

        my_sum_utils::register_impl::<MySum, _>(connection).unwrap();

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

        range_max_utils::register_impl::<RangeMax<i32>, _, _, _>(connection).unwrap();
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
}
