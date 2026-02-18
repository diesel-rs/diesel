#[cfg(not(all(target_family = "wasm", target_os = "unknown")))]
extern crate libsqlite3_sys as ffi;

#[cfg(all(target_family = "wasm", target_os = "unknown"))]
use sqlite_wasm_rs as ffi;

mod authorizer;
mod bind_collector;
mod functions;
mod owned_row;
mod raw;
mod row;
mod serialized_database;
mod sqlite_value;
mod statement_iterator;
mod stmt;
mod trace;
mod update_hook;

pub use self::authorizer::{AuthorizerAction, AuthorizerContext, AuthorizerDecision};
pub(in crate::sqlite) use self::bind_collector::SqliteBindCollector;
pub use self::bind_collector::SqliteBindValue;
pub use self::serialized_database::SerializedDatabase;
pub use self::sqlite_value::SqliteValue;
pub use self::trace::{SqliteTraceEvent, SqliteTraceFlags};
pub use self::update_hook::{ChangeHookId, SqliteChangeEvent, SqliteChangeOp, SqliteChangeOps};

use self::raw::RawConnection;
use self::statement_iterator::*;
use self::stmt::{Statement, StatementUse};
use super::SqliteAggregateFunction;
use crate::connection::instrumentation::{DynInstrumentation, StrQueryHelper};
use crate::connection::statement_cache::StatementCache;
use crate::connection::*;
use crate::deserialize::{FromSqlRow, StaticallySizedRow};
use crate::expression::QueryMetadata;
use crate::query_builder::nodes::{Identifier, StaticQueryFragment};
use crate::query_builder::*;
use crate::result::*;
use crate::serialize::ToSql;
use crate::sql_types::{HasSqlType, TypeMetadata};
use crate::sqlite::Sqlite;
use alloc::boxed::Box;
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

    /// Registers a callback invoked for any row change (insert, update, or
    /// delete) on any table, filtered by the given [`SqliteChangeOps`] mask.
    ///
    /// This is the untyped variant — the table name is provided as a string
    /// in [`SqliteChangeEvent::table_name`], suitable for logging all changes
    /// regardless of table.
    ///
    /// The callback fires **synchronously during `sqlite3_step()`**, i.e.
    /// while the triggering statement is executing. The connection is **not**
    /// available inside the callback.
    ///
    /// Multiple hooks can be registered on the same connection (and even
    /// the same table); they fire in registration order.
    ///
    /// Returns a [`ChangeHookId`] that can be used to remove the hook later
    /// via [`remove_change_hook`](Self::remove_change_hook). You can also
    /// remove all hooks for a specific table with
    /// [`clear_change_hooks_for`](Self::clear_change_hooks_for), or remove
    /// every hook at once with
    /// [`clear_all_change_hooks`](Self::clear_all_change_hooks).
    ///
    /// # Limitations
    ///
    /// These limitations come from the underlying
    /// [`sqlite3_update_hook`](https://www.sqlite.org/c3ref/update_hook.html)
    /// API:
    ///
    /// - Only fires for [rowid tables](https://www.sqlite.org/rowidtable.html).
    ///   [`WITHOUT ROWID`](https://www.sqlite.org/withoutrowid.html) tables do
    ///   not trigger this callback.
    /// - Not invoked for changes to internal system tables (e.g.
    ///   `sqlite_sequence`).
    /// - Not invoked for implicit deletes caused by
    ///   [`ON CONFLICT REPLACE`](https://www.sqlite.org/lang_conflict.html).
    /// - Not invoked for deletes using the
    ///   [truncate optimization](https://www.sqlite.org/lang_delete.html#truncateopt).
    ///
    /// # Example
    ///
    /// ```rust
    /// use diesel::prelude::*;
    /// use diesel::sqlite::{SqliteConnection, SqliteChangeOps, SqliteChangeOp};
    /// use std::sync::{Arc, Mutex};
    ///
    /// diesel::table! {
    ///     users (id) {
    ///         id -> Integer,
    ///         name -> Text,
    ///     }
    /// }
    ///
    /// #[derive(Insertable)]
    /// #[diesel(table_name = users)]
    /// struct NewUser<'a> {
    ///     name: &'a str,
    /// }
    ///
    /// # use diesel::connection::SimpleConnection;
    /// # let conn = &mut SqliteConnection::establish(":memory:").unwrap();
    /// # conn.batch_execute("CREATE TABLE users (id INTEGER PRIMARY KEY, name TEXT NOT NULL)").unwrap();
    /// let log = Arc::new(Mutex::new(Vec::new()));
    /// let log2 = log.clone();
    ///
    /// conn.on_change(SqliteChangeOps::ALL, move |event| {
    ///     log2.lock().unwrap().push((
    ///         event.op,
    ///         event.table_name.to_owned(),
    ///         event.rowid,
    ///     ));
    /// });
    ///
    /// diesel::insert_into(users::table)
    ///     .values(&NewUser { name: "Alice" })
    ///     .execute(conn)
    ///     .unwrap();
    ///
    /// let entries = log.lock().unwrap();
    /// assert_eq!(entries.len(), 1);
    /// assert_eq!(entries[0].0, SqliteChangeOp::Insert);
    /// assert_eq!(entries[0].1, "users");
    /// ```
    pub fn on_change<F>(&mut self, ops: SqliteChangeOps, hook: F) -> ChangeHookId
    where
        F: FnMut(SqliteChangeEvent<'_>) + Send + 'static,
    {
        self.register_change_hook(None, ops, Box::new(hook))
    }

    /// Registers a callback invoked when a row is inserted into table `T`.
    ///
    /// The generic parameter `T` is a [`table!`]-generated table type (e.g.
    /// `users::table`). The table name is extracted at registration time via
    /// [`StaticQueryFragment`].
    ///
    /// The callback fires synchronously during `sqlite3_step()`.
    /// The connection is **not** available inside the callback.
    ///
    /// Multiple hooks can be registered on the same table; they fire in
    /// registration order. The returned [`ChangeHookId`] can be passed to
    /// [`remove_change_hook`](Self::remove_change_hook) to remove just this
    /// hook, or use [`clear_change_hooks_for`](Self::clear_change_hooks_for)
    /// / [`clear_all_change_hooks`](Self::clear_all_change_hooks) to remove
    /// hooks in bulk.
    ///
    /// # Example
    ///
    /// ```rust
    /// use diesel::prelude::*;
    /// use diesel::sqlite::SqliteConnection;
    /// use std::sync::{Arc, Mutex};
    ///
    /// diesel::table! {
    ///     users (id) {
    ///         id -> Integer,
    ///         name -> Text,
    ///     }
    /// }
    ///
    /// # use diesel::connection::SimpleConnection;
    /// # let conn = &mut SqliteConnection::establish(":memory:").unwrap();
    /// # conn.batch_execute("CREATE TABLE users (id INTEGER PRIMARY KEY, name TEXT NOT NULL)").unwrap();
    /// let ids = Arc::new(Mutex::new(Vec::new()));
    /// let ids2 = ids.clone();
    ///
    /// conn.on_insert::<users::table, _>(move |event| {
    ///     ids2.lock().unwrap().push(event.rowid);
    /// });
    ///
    /// diesel::insert_into(users::table)
    ///     .values(users::name.eq("Alice"))
    ///     .execute(conn)
    ///     .unwrap();
    ///
    /// assert_eq!(*ids.lock().unwrap(), vec![1i64]);
    /// ```
    pub fn on_insert<T, F>(&mut self, hook: F) -> ChangeHookId
    where
        T: StaticQueryFragment<Component = Identifier<'static>>,
        F: FnMut(SqliteChangeEvent<'_>) + Send + 'static,
    {
        let table_name = T::STATIC_COMPONENT.0;
        self.register_change_hook(Some(table_name), SqliteChangeOps::INSERT, Box::new(hook))
    }

    /// Registers a callback invoked when a row is updated in table `T`.
    /// See [`on_insert`](Self::on_insert) for details on multiplicity,
    /// dispatch timing, and removal.
    ///
    /// # Example
    ///
    /// ```rust
    /// use diesel::prelude::*;
    /// use diesel::sqlite::SqliteConnection;
    /// use std::sync::{Arc, Mutex};
    ///
    /// diesel::table! {
    ///     users (id) {
    ///         id -> Integer,
    ///         name -> Text,
    ///     }
    /// }
    ///
    /// # use diesel::connection::SimpleConnection;
    /// # let conn = &mut SqliteConnection::establish(":memory:").unwrap();
    /// # conn.batch_execute("CREATE TABLE users (id INTEGER PRIMARY KEY, name TEXT NOT NULL)").unwrap();
    /// let count = Arc::new(Mutex::new(0u32));
    /// let count2 = count.clone();
    ///
    /// conn.on_update::<users::table, _>(move |_event| {
    ///     *count2.lock().unwrap() += 1;
    /// });
    ///
    /// diesel::insert_into(users::table)
    ///     .values(users::name.eq("Alice"))
    ///     .execute(conn).unwrap();
    /// // Insert does not trigger the update hook.
    /// assert_eq!(*count.lock().unwrap(), 0);
    ///
    /// diesel::update(users::table.filter(users::id.eq(1)))
    ///     .set(users::name.eq("Bob"))
    ///     .execute(conn).unwrap();
    /// assert_eq!(*count.lock().unwrap(), 1);
    /// ```
    pub fn on_update<T, F>(&mut self, hook: F) -> ChangeHookId
    where
        T: StaticQueryFragment<Component = Identifier<'static>>,
        F: FnMut(SqliteChangeEvent<'_>) + Send + 'static,
    {
        let table_name = T::STATIC_COMPONENT.0;
        self.register_change_hook(Some(table_name), SqliteChangeOps::UPDATE, Box::new(hook))
    }

    /// Registers a callback invoked when a row is deleted from table `T`.
    /// See [`on_insert`](Self::on_insert) for details on multiplicity,
    /// dispatch timing, and removal.
    ///
    /// # Example
    ///
    /// ```rust
    /// use diesel::prelude::*;
    /// use diesel::sqlite::SqliteConnection;
    /// use std::sync::{Arc, Mutex};
    ///
    /// diesel::table! {
    ///     users (id) {
    ///         id -> Integer,
    ///         name -> Text,
    ///     }
    /// }
    ///
    /// # use diesel::connection::SimpleConnection;
    /// # let conn = &mut SqliteConnection::establish(":memory:").unwrap();
    /// # conn.batch_execute("CREATE TABLE users (id INTEGER PRIMARY KEY, name TEXT NOT NULL)").unwrap();
    /// let deleted = Arc::new(Mutex::new(Vec::new()));
    /// let deleted2 = deleted.clone();
    ///
    /// conn.on_delete::<users::table, _>(move |event| {
    ///     deleted2.lock().unwrap().push(event.rowid);
    /// });
    ///
    /// diesel::insert_into(users::table)
    ///     .values(users::name.eq("Alice"))
    ///     .execute(conn).unwrap();
    /// diesel::delete(users::table.filter(users::id.eq(1)))
    ///     .execute(conn).unwrap();
    ///
    /// assert_eq!(*deleted.lock().unwrap(), vec![1i64]);
    /// ```
    pub fn on_delete<T, F>(&mut self, hook: F) -> ChangeHookId
    where
        T: StaticQueryFragment<Component = Identifier<'static>>,
        F: FnMut(SqliteChangeEvent<'_>) + Send + 'static,
    {
        let table_name = T::STATIC_COMPONENT.0;
        self.register_change_hook(Some(table_name), SqliteChangeOps::DELETE, Box::new(hook))
    }

    /// Removes a previously registered change hook by its [`ChangeHookId`].
    ///
    /// Returns `true` if a hook with that ID was found and removed, `false`
    /// if no hook matched (e.g. it was already removed).
    ///
    /// # Example
    ///
    /// ```rust
    /// use diesel::prelude::*;
    /// use diesel::sqlite::SqliteConnection;
    /// use std::sync::{Arc, Mutex};
    ///
    /// diesel::table! {
    ///     users (id) {
    ///         id -> Integer,
    ///         name -> Text,
    ///     }
    /// }
    ///
    /// # use diesel::connection::SimpleConnection;
    /// # let conn = &mut SqliteConnection::establish(":memory:").unwrap();
    /// # conn.batch_execute("CREATE TABLE users (id INTEGER PRIMARY KEY, name TEXT NOT NULL)").unwrap();
    /// let count = Arc::new(Mutex::new(0u32));
    /// let count2 = count.clone();
    ///
    /// let id = conn.on_insert::<users::table, _>(move |_| {
    ///     *count2.lock().unwrap() += 1;
    /// });
    ///
    /// diesel::insert_into(users::table)
    ///     .values(users::name.eq("Alice"))
    ///     .execute(conn).unwrap();
    /// assert_eq!(*count.lock().unwrap(), 1);
    ///
    /// assert!(conn.remove_change_hook(id));
    ///
    /// diesel::insert_into(users::table)
    ///     .values(users::name.eq("Bob"))
    ///     .execute(conn).unwrap();
    /// // Still 1 — the hook was removed.
    /// assert_eq!(*count.lock().unwrap(), 1);
    /// ```
    pub fn remove_change_hook(&mut self, id: ChangeHookId) -> bool {
        let mut hooks = self.raw_connection.change_hooks.borrow_mut();
        let removed = hooks.remove(id);
        let is_empty = hooks.is_empty();
        drop(hooks);
        if is_empty {
            self.raw_connection.unregister_raw_update_hook();
        }
        removed
    }

    /// Removes all change hooks registered for table `T`.
    ///
    /// # Example
    ///
    /// ```rust
    /// use diesel::prelude::*;
    /// use diesel::sqlite::SqliteConnection;
    /// use std::sync::{Arc, Mutex};
    ///
    /// diesel::table! {
    ///     users (id) {
    ///         id -> Integer,
    ///         name -> Text,
    ///     }
    /// }
    ///
    /// diesel::table! {
    ///     posts (id) {
    ///         id -> Integer,
    ///         title -> Text,
    ///     }
    /// }
    ///
    /// # use diesel::connection::SimpleConnection;
    /// # let conn = &mut SqliteConnection::establish(":memory:").unwrap();
    /// # conn.batch_execute("CREATE TABLE users (id INTEGER PRIMARY KEY, name TEXT NOT NULL)").unwrap();
    /// # conn.batch_execute("CREATE TABLE posts (id INTEGER PRIMARY KEY, title TEXT NOT NULL)").unwrap();
    /// let user_count = Arc::new(Mutex::new(0u32));
    /// let post_count = Arc::new(Mutex::new(0u32));
    /// let uc = user_count.clone();
    /// let pc = post_count.clone();
    ///
    /// conn.on_insert::<users::table, _>(move |_| { *uc.lock().unwrap() += 1; });
    /// conn.on_insert::<posts::table, _>(move |_| { *pc.lock().unwrap() += 1; });
    ///
    /// conn.clear_change_hooks_for::<users::table>();
    ///
    /// diesel::insert_into(users::table)
    ///     .values(users::name.eq("Alice"))
    ///     .execute(conn).unwrap();
    /// diesel::insert_into(posts::table)
    ///     .values(posts::title.eq("Hello"))
    ///     .execute(conn).unwrap();
    ///
    /// assert_eq!(*user_count.lock().unwrap(), 0); // cleared
    /// assert_eq!(*post_count.lock().unwrap(), 1); // still active
    /// ```
    pub fn clear_change_hooks_for<T>(&mut self)
    where
        T: StaticQueryFragment<Component = Identifier<'static>>,
    {
        let table_name = T::STATIC_COMPONENT.0;
        let mut hooks = self.raw_connection.change_hooks.borrow_mut();
        hooks.clear_for_table(table_name);
        let is_empty = hooks.is_empty();
        drop(hooks);
        if is_empty {
            self.raw_connection.unregister_raw_update_hook();
        }
    }

    /// Removes all registered change hooks and unregisters the underlying
    /// [`sqlite3_update_hook`](https://www.sqlite.org/c3ref/update_hook.html).
    ///
    /// After calling this, no change-related callbacks will fire until new
    /// hooks are registered.
    ///
    /// # Example
    ///
    /// ```rust
    /// use diesel::prelude::*;
    /// use diesel::sqlite::SqliteConnection;
    /// use std::sync::{Arc, Mutex};
    ///
    /// diesel::table! {
    ///     users (id) {
    ///         id -> Integer,
    ///         name -> Text,
    ///     }
    /// }
    ///
    /// # use diesel::connection::SimpleConnection;
    /// # let conn = &mut SqliteConnection::establish(":memory:").unwrap();
    /// # conn.batch_execute("CREATE TABLE users (id INTEGER PRIMARY KEY, name TEXT NOT NULL)").unwrap();
    /// let count = Arc::new(Mutex::new(0u32));
    /// let count2 = count.clone();
    ///
    /// conn.on_insert::<users::table, _>(move |_| { *count2.lock().unwrap() += 1; });
    ///
    /// conn.clear_all_change_hooks();
    ///
    /// diesel::insert_into(users::table)
    ///     .values(users::name.eq("Alice"))
    ///     .execute(conn).unwrap();
    ///
    /// assert_eq!(*count.lock().unwrap(), 0);
    /// ```
    pub fn clear_all_change_hooks(&mut self) {
        self.raw_connection.change_hooks.borrow_mut().clear_all();
        self.raw_connection.unregister_raw_update_hook();
    }

    fn register_change_hook(
        &mut self,
        table_name: Option<&'static str>,
        ops: SqliteChangeOps,
        hook: Box<dyn FnMut(SqliteChangeEvent<'_>) + Send>,
    ) -> ChangeHookId {
        self.raw_connection.register_raw_update_hook();
        self.raw_connection
            .change_hooks
            .borrow_mut()
            .add(table_name, ops, hook)
    }

    /// Loads a single row from table `T` by its SQLite `rowid`.
    ///
    /// This is useful for retrieving the full row data after receiving a
    /// [`SqliteChangeEvent`] from a change hook, since the event only
    /// contains the `rowid` of the affected row.
    ///
    /// If the row cannot be found (e.g. it was deleted by a trigger), this
    /// returns [`Err(NotFound)`](crate::result::Error::NotFound).
    ///
    /// This is NOT a shortcut for loading by primary key.
    /// It always queries by `rowid`, which may not be the same column
    /// as the primary key, or may not even exist for [`WITHOUT ROWID`](https://www.sqlite.org/withoutrowid.html) tables.
    /// The caller must ensure that the provided `rowid` is correct
    /// for the intended query.
    ///
    /// # Example
    ///
    /// ```rust
    /// use diesel::prelude::*;
    /// use diesel::sqlite::SqliteConnection;
    ///
    /// diesel::table! {
    ///     users (id) {
    ///         id -> Integer,
    ///         name -> Text,
    ///     }
    /// }
    ///
    /// #[derive(Queryable)]
    /// struct User {
    ///     id: i32,
    ///     name: String,
    /// }
    ///
    /// # use diesel::connection::SimpleConnection;
    /// # let conn = &mut SqliteConnection::establish(":memory:").unwrap();
    /// # conn.batch_execute("CREATE TABLE users (id INTEGER PRIMARY KEY, name TEXT NOT NULL)").unwrap();
    /// diesel::insert_into(users::table)
    ///     .values(users::name.eq("Alice"))
    ///     .execute(conn)
    ///     .unwrap();
    ///
    /// let user: User = conn.find_by_rowid::<users::table, _>(1).unwrap();
    /// assert_eq!(user.name, "Alice");
    /// ```
    pub fn find_by_rowid<T, M>(&mut self, rowid: i64) -> QueryResult<M>
    where
        T: crate::Table
            + crate::associations::HasTable<Table = T>
            + crate::query_dsl::methods::FilterDsl<
                crate::expression::UncheckedBind<
                    crate::expression::SqlLiteral<crate::sql_types::Bool>,
                    crate::expression::bound::Bound<crate::sql_types::BigInt, i64>,
                >,
            >,
        crate::dsl::Filter<
            T,
            crate::expression::UncheckedBind<
                crate::expression::SqlLiteral<crate::sql_types::Bool>,
                crate::expression::bound::Bound<crate::sql_types::BigInt, i64>,
            >,
        >: crate::query_dsl::RunQueryDsl<SqliteConnection> + crate::query_dsl::methods::LimitDsl,
        crate::dsl::Limit<
            crate::dsl::Filter<
                T,
                crate::expression::UncheckedBind<
                    crate::expression::SqlLiteral<crate::sql_types::Bool>,
                    crate::expression::bound::Bound<crate::sql_types::BigInt, i64>,
                >,
            >,
        >: crate::query_dsl::LoadQuery<'static, SqliteConnection, M>,
    {
        use crate::query_dsl::RunQueryDsl;
        T::table()
            .filter(
                crate::dsl::sql::<crate::sql_types::Bool>("rowid = ")
                    .bind::<crate::sql_types::BigInt, _>(rowid),
            )
            .first::<M>(self)
    }

    /// Registers a callback invoked when a transaction is about to be
    /// committed.
    ///
    /// The callback returns a `bool`:
    /// - `false` — the commit proceeds normally.
    /// - `true` — the commit is converted into a rollback.
    ///
    /// Only one commit hook can be active at a time per connection.
    /// Registering a new one replaces the previous.
    ///
    /// The callback must not use the database connection. Panics in the
    /// callback abort the process.
    ///
    /// See: [`sqlite3_commit_hook`](https://www.sqlite.org/c3ref/commit_hook.html)
    ///
    /// # Example
    ///
    /// ```rust
    /// use diesel::prelude::*;
    /// use diesel::sqlite::SqliteConnection;
    /// use std::sync::{Arc, Mutex};
    ///
    /// diesel::table! {
    ///     users (id) {
    ///         id -> Integer,
    ///         name -> Text,
    ///     }
    /// }
    ///
    /// # use diesel::connection::SimpleConnection;
    /// # let conn = &mut SqliteConnection::establish(":memory:").unwrap();
    /// # conn.batch_execute("CREATE TABLE users (id INTEGER PRIMARY KEY, name TEXT NOT NULL)").unwrap();
    /// let commits = Arc::new(Mutex::new(0u32));
    /// let commits2 = commits.clone();
    ///
    /// conn.on_commit(move || {
    ///     *commits2.lock().unwrap() += 1;
    ///     false // allow the commit to proceed
    /// });
    ///
    /// conn.immediate_transaction(|conn| {
    ///     diesel::insert_into(users::table)
    ///         .values(users::name.eq("Alice"))
    ///         .execute(conn)?;
    ///     Ok::<_, diesel::result::Error>(())
    /// }).unwrap();
    ///
    /// assert_eq!(*commits.lock().unwrap(), 1);
    /// ```
    pub fn on_commit<F>(&mut self, hook: F)
    where
        F: FnMut() -> bool + Send + 'static,
    {
        self.raw_connection.set_commit_hook(hook);
    }

    /// Removes the commit hook. Subsequent commits will not invoke any
    /// callback.
    ///
    /// # Example
    ///
    /// ```rust
    /// use diesel::prelude::*;
    /// use diesel::sqlite::SqliteConnection;
    /// use std::sync::{Arc, Mutex};
    ///
    /// diesel::table! {
    ///     users (id) {
    ///         id -> Integer,
    ///         name -> Text,
    ///     }
    /// }
    ///
    /// # use diesel::connection::SimpleConnection;
    /// # let conn = &mut SqliteConnection::establish(":memory:").unwrap();
    /// # conn.batch_execute("CREATE TABLE users (id INTEGER PRIMARY KEY, name TEXT NOT NULL)").unwrap();
    /// let count = Arc::new(Mutex::new(0u32));
    /// let count2 = count.clone();
    /// conn.on_commit(move || { *count2.lock().unwrap() += 1; false });
    ///
    /// conn.remove_commit_hook();
    ///
    /// conn.immediate_transaction(|conn| {
    ///     diesel::insert_into(users::table)
    ///         .values(users::name.eq("Alice"))
    ///         .execute(conn)?;
    ///     Ok::<_, diesel::result::Error>(())
    /// }).unwrap();
    ///
    /// assert_eq!(*count.lock().unwrap(), 0);
    /// ```
    pub fn remove_commit_hook(&mut self) {
        self.raw_connection.remove_commit_hook();
    }

    /// Registers a callback invoked after a transaction is rolled back.
    ///
    /// This is **not** invoked for the implicit rollback that occurs when
    /// the connection is closed. It **is** invoked when a commit hook
    /// forces a rollback by returning `true`.
    ///
    /// Only one rollback hook can be active at a time per connection.
    ///
    /// The callback must not use the database connection. Panics in the
    /// callback abort the process.
    ///
    /// See: [`sqlite3_rollback_hook`](https://www.sqlite.org/c3ref/commit_hook.html)
    ///
    /// # Example
    ///
    /// ```rust
    /// # use diesel::prelude::*;
    /// # use diesel::connection::SimpleConnection;
    /// # use diesel::sqlite::SqliteConnection;
    /// # let conn = &mut SqliteConnection::establish(":memory:").unwrap();
    /// # conn.batch_execute("CREATE TABLE users (id INTEGER PRIMARY KEY, name TEXT NOT NULL)").unwrap();
    /// use std::sync::{Arc, Mutex};
    ///
    /// let rollbacks = Arc::new(Mutex::new(0u32));
    /// let rb2 = rollbacks.clone();
    ///
    /// conn.on_rollback(move || {
    ///     *rb2.lock().unwrap() += 1;
    /// });
    ///
    /// // Force a rollback by returning an error.
    /// let _ = conn.immediate_transaction(|_conn| {
    ///     Err::<(), _>(diesel::result::Error::RollbackTransaction)
    /// });
    ///
    /// assert_eq!(*rollbacks.lock().unwrap(), 1);
    /// ```
    pub fn on_rollback<F>(&mut self, hook: F)
    where
        F: FnMut() + Send + 'static,
    {
        self.raw_connection.set_rollback_hook(hook);
    }

    /// Removes the rollback hook.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use diesel::prelude::*;
    /// # use diesel::connection::SimpleConnection;
    /// # use diesel::sqlite::SqliteConnection;
    /// # let conn = &mut SqliteConnection::establish(":memory:").unwrap();
    /// use std::sync::{Arc, Mutex};
    ///
    /// let count = Arc::new(Mutex::new(0u32));
    /// let count2 = count.clone();
    /// conn.on_rollback(move || { *count2.lock().unwrap() += 1; });
    ///
    /// conn.remove_rollback_hook();
    ///
    /// let _ = conn.immediate_transaction(|_conn| {
    ///     Err::<(), _>(diesel::result::Error::RollbackTransaction)
    /// });
    ///
    /// assert_eq!(*count.lock().unwrap(), 0);
    /// ```
    pub fn remove_rollback_hook(&mut self) {
        self.raw_connection.remove_rollback_hook();
    }

    /// Registers a callback invoked after a commit completes in
    /// [WAL mode](https://www.sqlite.org/wal.html).
    ///
    /// The callback receives:
    /// - `db_name` — the database name (`"main"`, `"temp"`, or an `ATTACH` alias).
    /// - `n_pages` — the number of pages currently in the WAL file.
    ///
    /// Useful for triggering custom checkpoint logic via
    /// [`PRAGMA wal_checkpoint`](https://www.sqlite.org/pragma.html#pragma_wal_checkpoint).
    ///
    /// Only one WAL hook can be active at a time per connection.
    ///
    /// **Warning:** [`PRAGMA wal_autocheckpoint`](https://www.sqlite.org/pragma.html#pragma_wal_autocheckpoint)
    /// and `sqlite3_wal_autocheckpoint()` internally call `sqlite3_wal_hook()`
    /// and will **overwrite** this hook.
    ///
    /// See: [`sqlite3_wal_hook`](https://www.sqlite.org/c3ref/wal_hook.html)
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use diesel::prelude::*;
    /// # use diesel::connection::SimpleConnection;
    /// # use diesel::sqlite::SqliteConnection;
    /// # let conn = &mut SqliteConnection::establish("test.db").unwrap();
    /// # conn.batch_execute("PRAGMA journal_mode = WAL;").unwrap();
    /// conn.on_wal(|db_name, n_pages| {
    ///     println!("WAL for {db_name}: {n_pages} pages");
    /// });
    /// ```
    pub fn on_wal<F>(&mut self, hook: F)
    where
        F: FnMut(&str, i32) + Send + 'static,
    {
        self.raw_connection.set_wal_hook(hook);
    }

    /// Removes the WAL hook.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use diesel::prelude::*;
    /// # use diesel::connection::SimpleConnection;
    /// # use diesel::sqlite::SqliteConnection;
    /// # let conn = &mut SqliteConnection::establish("test.db").unwrap();
    /// # conn.batch_execute("PRAGMA journal_mode = WAL;").unwrap();
    /// conn.on_wal(|_db, _pages| { /* ... */ });
    /// conn.remove_wal_hook();
    /// ```
    pub fn remove_wal_hook(&mut self) {
        self.raw_connection.remove_wal_hook();
    }

    /// Registers a progress handler for long-running query interruption.
    /// Added in SQLite 3.0.0 (June 2004).
    ///
    /// The callback is invoked periodically during long-running SQL queries.
    /// `n` is the approximate number of VM instructions between callbacks.
    /// **Suggested minimum**: 1000 (at N=1, overhead is ~1.5μs per callback).
    ///
    /// Return `true` to interrupt the query (causes `SQLITE_INTERRUPT`),
    /// or `false` to continue.
    ///
    /// Only one progress handler can be active at a time per connection.
    /// Registering a new one replaces the previous.
    ///
    /// **Callback restriction**: The callback must not use the database
    /// connection. Since SQLite 3.41.0, the handler may also fire during
    /// `sqlite3_prepare()` for complex queries.
    ///
    /// Panics in the callback abort the process.
    ///
    /// See: [`sqlite3_progress_handler`](https://sqlite.org/c3ref/progress_handler.html)
    ///
    /// # Example
    ///
    /// ```rust
    /// # use diesel::prelude::*;
    /// # use diesel::sqlite::SqliteConnection;
    /// use std::sync::atomic::{AtomicBool, Ordering};
    /// use std::sync::Arc;
    ///
    /// # let conn = &mut SqliteConnection::establish(":memory:").unwrap();
    /// let cancelled = Arc::new(AtomicBool::new(false));
    /// let cancelled_clone = cancelled.clone();
    ///
    /// conn.on_progress(1000, move || {
    ///     cancelled_clone.load(Ordering::Relaxed)
    /// });
    ///
    /// // Later: remove the handler
    /// conn.remove_progress_handler();
    /// ```
    pub fn on_progress<F>(&mut self, n: i32, hook: F)
    where
        F: FnMut() -> bool + Send + 'static,
    {
        self.raw_connection.set_progress_handler(n, hook);
    }

    /// Removes the progress handler.
    ///
    /// See [`on_progress`](Self::on_progress) for usage example.
    pub fn remove_progress_handler(&mut self) {
        self.raw_connection.remove_progress_handler();
    }

    /// Registers a custom busy handler for lock contention.
    /// Added in SQLite 3.0.0 (June 2004).
    ///
    /// The callback receives the retry count (starting from 0) and returns
    /// `true` to retry, `false` to abort (returns `SQLITE_BUSY` to caller).
    ///
    /// **Warning**: Setting this clears any `busy_timeout` previously set.
    /// Conversely, calling `set_busy_timeout` clears this handler.
    ///
    /// Only one busy handler can be active at a time per connection.
    ///
    /// **Callback restriction**: The callback must not use the database
    /// connection. If the callback modifies the database, behavior is
    /// undefined. SQLite may return `SQLITE_BUSY` instead of calling the
    /// handler to prevent deadlocks.
    ///
    /// Panics in the callback abort the process.
    ///
    /// See: [`sqlite3_busy_handler`](https://sqlite.org/c3ref/busy_handler.html)
    ///
    /// # Example
    ///
    /// ```rust
    /// # use diesel::prelude::*;
    /// # use diesel::sqlite::SqliteConnection;
    /// use std::thread;
    /// use std::time::Duration;
    ///
    /// # let conn = &mut SqliteConnection::establish(":memory:").unwrap();
    /// conn.on_busy(|retry_count| {
    ///     if retry_count < 5 {
    ///         thread::sleep(Duration::from_millis(100));
    ///         true // retry
    ///     } else {
    ///         false // give up
    ///     }
    /// });
    ///
    /// // Later: remove the handler
    /// conn.remove_busy_handler();
    /// ```
    pub fn on_busy<F>(&mut self, hook: F)
    where
        F: FnMut(i32) -> bool + Send + 'static,
    {
        self.raw_connection.set_busy_handler(hook);
    }

    /// Removes the custom busy handler.
    ///
    /// See [`on_busy`](Self::on_busy) for usage example.
    pub fn remove_busy_handler(&mut self) {
        self.raw_connection.remove_busy_handler();
    }

    /// Sets a simple timeout-based busy handler.
    /// Added in SQLite 3.0.0 (June 2004).
    ///
    /// When a table is locked, SQLite will sleep and retry until `ms`
    /// milliseconds have elapsed. Pass 0 to disable (return `SQLITE_BUSY`
    /// immediately).
    ///
    /// **Note**: Setting this clears any custom [`on_busy`](Self::on_busy)
    /// handler. Conversely, calling `on_busy` clears this timeout.
    ///
    /// For most use cases, this is simpler than a custom busy handler.
    ///
    /// See: [`sqlite3_busy_timeout`](https://sqlite.org/c3ref/busy_timeout.html)
    ///
    /// # Example
    ///
    /// ```rust
    /// # use diesel::prelude::*;
    /// # use diesel::sqlite::SqliteConnection;
    /// # let conn = &mut SqliteConnection::establish(":memory:").unwrap();
    /// // Wait up to 5 seconds for locked tables
    /// conn.set_busy_timeout(5000);
    /// ```
    pub fn set_busy_timeout(&mut self, ms: i32) {
        self.raw_connection.set_busy_timeout(ms);
    }

    /// Registers an authorizer callback for SQL compilation access control.
    /// Added in SQLite 3.0.0 (June 2004).
    ///
    /// The callback is invoked during `sqlite3_prepare()` (statement
    /// compilation) to control access to database objects. It receives
    /// an [`AuthorizerContext`] describing the operation and returns an
    /// [`AuthorizerDecision`].
    ///
    /// **Security Note**: The authorizer is only called during statement
    /// compilation, NOT during execution. It cannot prevent all attack
    /// vectors (e.g., CVE-2015-3659 showed bypasses via fts3_tokenizer).
    /// Use it as defense-in-depth, not as a sole security mechanism.
    ///
    /// **Schema changes**: The authorizer may be re-invoked during
    /// `sqlite3_step()` if schema changes trigger statement recompilation.
    ///
    /// **Callback restriction**: The callback must not modify the database
    /// connection.
    ///
    /// Only one authorizer can be active at a time per connection.
    /// Panics in the callback abort the process.
    ///
    /// See: [`sqlite3_set_authorizer`](https://sqlite.org/c3ref/set_authorizer.html)
    ///
    /// # Example
    ///
    /// ```rust
    /// # use diesel::prelude::*;
    /// use diesel::sqlite::{
    ///     SqliteConnection, AuthorizerContext, AuthorizerDecision, AuthorizerAction,
    /// };
    ///
    /// # let conn = &mut SqliteConnection::establish(":memory:").unwrap();
    /// conn.on_authorize(|ctx| {
    ///     match ctx.action {
    ///         AuthorizerAction::Delete => AuthorizerDecision::Deny,
    ///         AuthorizerAction::DropTable | AuthorizerAction::DropIndex => {
    ///             AuthorizerDecision::Deny
    ///         }
    ///         _ => AuthorizerDecision::Allow,
    ///     }
    /// });
    ///
    /// // Later: remove the authorizer
    /// conn.remove_authorizer();
    /// ```
    pub fn on_authorize<F>(&mut self, hook: F)
    where
        F: FnMut(AuthorizerContext<'_>) -> AuthorizerDecision + Send + 'static,
    {
        self.raw_connection.set_authorizer(hook);
    }

    /// Removes the authorizer callback.
    ///
    /// See [`on_authorize`](Self::on_authorize) for usage example.
    pub fn remove_authorizer(&mut self) {
        self.raw_connection.remove_authorizer();
    }

    /// Registers a trace callback for SQL execution monitoring.
    /// Added in SQLite 3.14.0 (2016-08-08).
    ///
    /// The callback receives events based on the provided [`SqliteTraceFlags`]
    /// mask. Available event types:
    ///
    /// - `STMT` — Statement start (receives SQL text)
    /// - `PROFILE` — Statement complete (receives SQL text and elapsed time)
    /// - `ROW` — Each row returned (no data, very frequent!)
    /// - `CLOSE` — Connection close
    ///
    /// **Performance Warning**: `SQLITE_TRACE_ROW` fires for EVERY row
    /// returned by a query. For large result sets, this can significantly
    /// impact performance. Prefer `STMT` and `PROFILE` for most logging
    /// use cases.
    ///
    /// Only one trace callback can be active at a time per connection.
    /// Registering a new one replaces the previous.
    ///
    /// Panics in the callback abort the process.
    ///
    /// See: [`sqlite3_trace_v2`](https://sqlite.org/c3ref/trace_v2.html)
    ///
    /// # Example
    ///
    /// ```rust
    /// # use diesel::prelude::*;
    /// use diesel::sqlite::{SqliteConnection, SqliteTraceFlags, SqliteTraceEvent};
    ///
    /// # let conn = &mut SqliteConnection::establish(":memory:").unwrap();
    /// conn.on_trace(SqliteTraceFlags::STMT | SqliteTraceFlags::PROFILE, |event| {
    ///     match event {
    ///         SqliteTraceEvent::Statement { sql, readonly } => {
    ///             println!("Executing ({}): {}", if readonly { "read" } else { "write" }, sql);
    ///         }
    ///         SqliteTraceEvent::Profile { sql, duration_ns, .. } => {
    ///             println!("{} took {} ns", sql, duration_ns);
    ///         }
    ///         _ => {}
    ///     }
    /// });
    ///
    /// // Later: remove the trace callback
    /// conn.remove_trace();
    /// ```
    pub fn on_trace<F>(&mut self, mask: SqliteTraceFlags, hook: F)
    where
        F: FnMut(SqliteTraceEvent<'_>) + Send + 'static,
    {
        self.raw_connection.set_trace(mask, hook);
    }

    /// Removes the trace callback.
    ///
    /// See [`on_trace`](Self::on_trace) for usage example.
    pub fn remove_trace(&mut self) {
        self.raw_connection.remove_trace();
    }

    /// Registers a callback for read-only statement execution (SELECT, read-only PRAGMA, etc.).
    ///
    /// This is a convenience wrapper around [`on_trace`](Self::on_trace) that filters for
    /// read-only statements only. Fires once per statement, not per row.
    ///
    /// # How it works
    ///
    /// Uses [`sqlite3_stmt_readonly`](https://sqlite.org/c3ref/stmt_readonly.html) to determine
    /// if a statement is read-only, which is more reliable than string matching.
    ///
    /// # What counts as read-only
    ///
    /// - `SELECT` statements (including `WITH ... SELECT`)
    /// - Read-only `PRAGMA` statements (e.g., `PRAGMA table_info(t)`)
    ///
    /// # What does NOT count as read-only
    ///
    /// - `INSERT`, `UPDATE`, `DELETE` statements
    /// - `CREATE`, `DROP`, `ALTER` statements
    /// - `WITH ... INSERT/UPDATE/DELETE ... RETURNING` (the outer statement modifies data)
    /// - Write `PRAGMA` statements (e.g., `PRAGMA foreign_keys = ON`)
    ///
    /// # Edge cases (not detected)
    ///
    /// - User-defined functions that modify the database via a separate connection
    ///   are **not** detected; the statement itself is still considered read-only
    /// - Virtual tables with write side effects are **not** detected
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use diesel::prelude::*;
    /// # use diesel::sqlite::SqliteConnection;
    /// let conn = &mut SqliteConnection::establish(":memory:").unwrap();
    ///
    /// conn.on_read(|sql| {
    ///     println!("Read query executed: {}", sql);
    /// });
    /// ```
    ///
    /// # Note
    ///
    /// This replaces any existing trace callback. If you need both `on_read` and
    /// custom tracing, use [`on_trace`](Self::on_trace) directly and check the
    /// `readonly` field on [`SqliteTraceEvent::Statement`].
    #[doc(alias = "on_trace")]
    pub fn on_read<F>(&mut self, mut hook: F)
    where
        F: FnMut(&str) + Send + 'static,
    {
        self.on_trace(SqliteTraceFlags::STMT, move |event| {
            if let SqliteTraceEvent::Statement { sql, readonly } = event
                && readonly
            {
                hook(sql);
            }
        });
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dsl::sql;
    use crate::prelude::*;
    use crate::sql_types::{Integer, Text};

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

    // ===================================================================
    // Change-hook integration tests
    // ===================================================================

    table! {
        hook_users {
            id -> Integer,
            name -> Text,
        }
    }

    table! {
        hook_posts {
            id -> Integer,
            title -> Text,
        }
    }

    fn setup_hook_tables(conn: &mut SqliteConnection) {
        crate::sql_query(
            "CREATE TABLE hook_users (id INTEGER PRIMARY KEY AUTOINCREMENT, name TEXT NOT NULL)",
        )
        .execute(conn)
        .unwrap();
        crate::sql_query(
            "CREATE TABLE hook_posts (id INTEGER PRIMARY KEY AUTOINCREMENT, title TEXT NOT NULL)",
        )
        .execute(conn)
        .unwrap();
    }

    #[diesel_test_helper::test]
    fn on_insert_fires_for_insert() {
        use std::sync::{Arc, Mutex};
        let conn = &mut connection();
        setup_hook_tables(conn);

        let fired = Arc::new(Mutex::new(Vec::new()));
        let fired2 = fired.clone();

        conn.on_insert::<hook_users::table, _>(move |event| {
            fired2.lock().unwrap().push((event.op, event.rowid));
        });

        // INSERT a row — the hook fires immediately during sqlite3_step().
        crate::sql_query("INSERT INTO hook_users (name) VALUES ('Alice')")
            .execute(conn)
            .unwrap();

        let events = fired.lock().unwrap().clone();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].0, SqliteChangeOp::Insert);
        assert_eq!(events[0].1, 1); // rowid
    }

    #[diesel_test_helper::test]
    fn on_delete_fires_only_for_delete() {
        use std::sync::{Arc, Mutex};
        let conn = &mut connection();
        setup_hook_tables(conn);

        let fired = Arc::new(Mutex::new(Vec::new()));
        let fired2 = fired.clone();

        conn.on_delete::<hook_users::table, _>(move |event| {
            fired2.lock().unwrap().push(event.op);
        });

        // INSERT + UPDATE + DELETE
        crate::sql_query("INSERT INTO hook_users (name) VALUES ('Alice')")
            .execute(conn)
            .unwrap();
        crate::sql_query("UPDATE hook_users SET name = 'Bob' WHERE id = 1")
            .execute(conn)
            .unwrap();
        crate::sql_query("DELETE FROM hook_users WHERE id = 1")
            .execute(conn)
            .unwrap();

        let events = fired.lock().unwrap().clone();
        // Only the DELETE should have matched.
        assert_eq!(events.len(), 1);
        assert_eq!(events[0], SqliteChangeOp::Delete);
    }

    #[diesel_test_helper::test]
    fn multiple_hooks_fire_in_order() {
        use std::sync::{Arc, Mutex};
        let conn = &mut connection();
        setup_hook_tables(conn);

        let order = Arc::new(Mutex::new(Vec::new()));
        let o1 = order.clone();
        let o2 = order.clone();

        conn.on_insert::<hook_users::table, _>(move |_| {
            o1.lock().unwrap().push(1);
        });
        conn.on_insert::<hook_users::table, _>(move |_| {
            o2.lock().unwrap().push(2);
        });

        crate::sql_query("INSERT INTO hook_users (name) VALUES ('X')")
            .execute(conn)
            .unwrap();

        assert_eq!(*order.lock().unwrap(), vec![1, 2]);
    }

    #[diesel_test_helper::test]
    fn remove_hook_prevents_dispatch() {
        use std::sync::{Arc, Mutex};
        let conn = &mut connection();
        setup_hook_tables(conn);

        let fired = Arc::new(Mutex::new(0u32));
        let f2 = fired.clone();

        let id = conn.on_insert::<hook_users::table, _>(move |_| {
            *f2.lock().unwrap() += 1;
        });

        crate::sql_query("INSERT INTO hook_users (name) VALUES ('A')")
            .execute(conn)
            .unwrap();
        assert_eq!(*fired.lock().unwrap(), 1);

        // Remove the hook.
        assert!(conn.remove_change_hook(id));

        crate::sql_query("INSERT INTO hook_users (name) VALUES ('B')")
            .execute(conn)
            .unwrap();
        // Should still be 1 — hook was removed.
        assert_eq!(*fired.lock().unwrap(), 1);
    }

    #[diesel_test_helper::test]
    fn events_fire_immediately_during_statement() {
        use std::sync::{Arc, Mutex};
        let conn = &mut connection();
        setup_hook_tables(conn);

        // Insert a row without a hook first.
        crate::sql_query("INSERT INTO hook_users (name) VALUES ('Z')")
            .execute(conn)
            .unwrap();

        let fired = Arc::new(Mutex::new(Vec::new()));
        let f2 = fired.clone();

        conn.on_update::<hook_users::table, _>(move |event| {
            f2.lock().unwrap().push(event.rowid);
        });

        // UPDATE triggers the C hook immediately during sqlite3_step().
        crate::sql_query("UPDATE hook_users SET name = 'W' WHERE id = 1")
            .execute(conn)
            .unwrap();

        assert_eq!(*fired.lock().unwrap(), vec![1i64]);
    }

    #[diesel_test_helper::test]
    fn on_update_fires_for_update_only() {
        use std::sync::{Arc, Mutex};
        let conn = &mut connection();
        setup_hook_tables(conn);

        let count = Arc::new(Mutex::new(0u32));
        let c2 = count.clone();

        conn.on_update::<hook_users::table, _>(move |event| {
            assert_eq!(event.op, SqliteChangeOp::Update);
            *c2.lock().unwrap() += 1;
        });

        crate::sql_query("INSERT INTO hook_users (name) VALUES ('A')")
            .execute(conn)
            .unwrap();
        crate::sql_query("UPDATE hook_users SET name = 'B' WHERE id = 1")
            .execute(conn)
            .unwrap();
        crate::sql_query("DELETE FROM hook_users WHERE id = 1")
            .execute(conn)
            .unwrap();

        assert_eq!(*count.lock().unwrap(), 1);
    }

    #[diesel_test_helper::test]
    fn on_change_catches_all_ops() {
        use std::sync::{Arc, Mutex};
        let conn = &mut connection();
        setup_hook_tables(conn);

        let events = Arc::new(Mutex::new(Vec::new()));
        let e2 = events.clone();

        conn.on_change(SqliteChangeOps::ALL, move |event| {
            e2.lock()
                .unwrap()
                .push((event.op, event.table_name.to_owned()));
        });

        crate::sql_query("INSERT INTO hook_users (name) VALUES ('A')")
            .execute(conn)
            .unwrap();
        crate::sql_query("INSERT INTO hook_posts (title) VALUES ('P')")
            .execute(conn)
            .unwrap();
        crate::sql_query("UPDATE hook_users SET name = 'B' WHERE id = 1")
            .execute(conn)
            .unwrap();
        crate::sql_query("DELETE FROM hook_posts WHERE id = 1")
            .execute(conn)
            .unwrap();

        let evts = events.lock().unwrap().clone();
        assert_eq!(evts.len(), 4);
        assert_eq!(evts[0], (SqliteChangeOp::Insert, "hook_users".to_owned()));
        assert_eq!(evts[1], (SqliteChangeOp::Insert, "hook_posts".to_owned()));
        assert_eq!(evts[2], (SqliteChangeOp::Update, "hook_users".to_owned()));
        assert_eq!(evts[3], (SqliteChangeOp::Delete, "hook_posts".to_owned()));
    }

    #[diesel_test_helper::test]
    fn on_change_with_partial_mask() {
        use std::sync::{Arc, Mutex};
        let conn = &mut connection();
        setup_hook_tables(conn);

        let count = Arc::new(Mutex::new(0u32));
        let c2 = count.clone();

        conn.on_change(
            SqliteChangeOps::INSERT | SqliteChangeOps::DELETE,
            move |_| {
                *c2.lock().unwrap() += 1;
            },
        );

        crate::sql_query("INSERT INTO hook_users (name) VALUES ('A')")
            .execute(conn)
            .unwrap();
        crate::sql_query("UPDATE hook_users SET name = 'B' WHERE id = 1")
            .execute(conn)
            .unwrap();
        crate::sql_query("DELETE FROM hook_users WHERE id = 1")
            .execute(conn)
            .unwrap();

        // INSERT + DELETE = 2, not UPDATE
        assert_eq!(*count.lock().unwrap(), 2);
    }

    #[diesel_test_helper::test]
    fn clear_hooks_for_table_only_removes_that_table() {
        use std::sync::{Arc, Mutex};
        let conn = &mut connection();
        setup_hook_tables(conn);

        let user_count = Arc::new(Mutex::new(0u32));
        let post_count = Arc::new(Mutex::new(0u32));
        let uc = user_count.clone();
        let pc = post_count.clone();

        conn.on_insert::<hook_users::table, _>(move |_| {
            *uc.lock().unwrap() += 1;
        });
        conn.on_insert::<hook_posts::table, _>(move |_| {
            *pc.lock().unwrap() += 1;
        });

        conn.clear_change_hooks_for::<hook_users::table>();

        crate::sql_query("INSERT INTO hook_users (name) VALUES ('X')")
            .execute(conn)
            .unwrap();
        crate::sql_query("INSERT INTO hook_posts (title) VALUES ('Y')")
            .execute(conn)
            .unwrap();

        assert_eq!(*user_count.lock().unwrap(), 0); // cleared
        assert_eq!(*post_count.lock().unwrap(), 1); // still active
    }

    #[diesel_test_helper::test]
    fn clear_all_removes_everything() {
        use std::sync::{Arc, Mutex};
        let conn = &mut connection();
        setup_hook_tables(conn);

        let count = Arc::new(Mutex::new(0u32));
        let c2 = count.clone();
        let c3 = count.clone();

        conn.on_insert::<hook_users::table, _>(move |_| {
            *c2.lock().unwrap() += 1;
        });
        conn.on_insert::<hook_posts::table, _>(move |_| {
            *c3.lock().unwrap() += 1;
        });

        conn.clear_all_change_hooks();

        crate::sql_query("INSERT INTO hook_users (name) VALUES ('X')")
            .execute(conn)
            .unwrap();
        crate::sql_query("INSERT INTO hook_posts (title) VALUES ('Y')")
            .execute(conn)
            .unwrap();

        assert_eq!(*count.lock().unwrap(), 0);
    }

    #[diesel_test_helper::test]
    fn hooks_fire_across_transactions() {
        use std::sync::{Arc, Mutex};
        let conn = &mut connection();
        setup_hook_tables(conn);

        let fired = Arc::new(Mutex::new(Vec::new()));
        let f2 = fired.clone();

        // Register hook BEFORE the transaction.
        conn.on_insert::<hook_users::table, _>(move |event| {
            f2.lock().unwrap().push(event.rowid);
        });

        conn.immediate_transaction(|conn| {
            crate::sql_query("INSERT INTO hook_users (name) VALUES ('TxUser')")
                .execute(conn)
                .unwrap();
            Ok::<_, crate::result::Error>(())
        })
        .unwrap();

        assert_eq!(fired.lock().unwrap().len(), 1);
    }

    #[derive(Debug, Clone, PartialEq, crate::Queryable)]
    struct HookUser {
        id: i32,
        name: String,
    }

    #[diesel_test_helper::test]
    fn find_by_rowid_loads_row() {
        let conn = &mut connection();
        setup_hook_tables(conn);

        crate::sql_query("INSERT INTO hook_users (name) VALUES ('Alice')")
            .execute(conn)
            .unwrap();

        let user: HookUser = conn.find_by_rowid::<hook_users::table, _>(1).unwrap();
        assert_eq!(user.name, "Alice");
    }

    #[diesel_test_helper::test]
    fn find_by_rowid_returns_not_found_for_missing_row() {
        let conn = &mut connection();
        setup_hook_tables(conn);

        let result = conn.find_by_rowid::<hook_users::table, HookUser>(999);
        assert!(matches!(result, Err(crate::result::Error::NotFound)));
    }

    table! {
        products {
            id -> Text,
            name -> Text,
            price -> Integer,
        }
    }

    #[derive(Debug, crate::Queryable)]
    struct Product {
        id: String,
        name: String,
        price: i32,
    }

    #[diesel_test_helper::test]
    fn find_by_rowid_with_text_pk_after_hook() {
        use std::sync::{Arc, Mutex};
        let conn = &mut connection();

        crate::sql_query(
            "CREATE TABLE products (id TEXT NOT NULL PRIMARY KEY, name TEXT NOT NULL, price INTEGER NOT NULL)",
        )
        .execute(conn)
        .unwrap();

        let inserted = Arc::new(Mutex::new(Vec::<i64>::new()));
        let inserted_hook = inserted.clone();

        conn.on_insert::<products::table, _>(move |event| {
            inserted_hook.lock().unwrap().push(event.rowid);
        });

        crate::sql_query("INSERT INTO products (id, name, price) VALUES ('sku-42', 'Widget', 999)")
            .execute(conn)
            .unwrap();

        let rowids = inserted.lock().unwrap();
        let rowid = rowids[0];

        let product: Product = conn.find_by_rowid::<products::table, _>(rowid).unwrap();

        assert_eq!(product.id, "sku-42");
        assert_eq!(product.name, "Widget");
        assert_eq!(product.price, 999);
    }

    // ===================================================================
    // Negative tests: cases where the update hook must NOT fire
    //
    // These are documented SQLite limitations of sqlite3_update_hook():
    // https://www.sqlite.org/c3ref/update_hook.html
    // ===================================================================

    /// The update hook is not invoked for WITHOUT ROWID tables.
    /// See: https://www.sqlite.org/c3ref/update_hook.html
    #[diesel_test_helper::test]
    fn update_hook_silent_for_without_rowid_tables() {
        use std::sync::{Arc, Mutex};
        let conn = &mut connection();

        crate::sql_query("CREATE TABLE kv (key TEXT PRIMARY KEY, val TEXT NOT NULL) WITHOUT ROWID")
            .execute(conn)
            .unwrap();

        let events: Arc<Mutex<Vec<SqliteChangeOp>>> = Arc::new(Mutex::new(Vec::new()));
        let e2 = events.clone();

        conn.on_change(SqliteChangeOps::ALL, move |ev| {
            if ev.table_name == "kv" {
                e2.lock().unwrap().push(ev.op);
            }
        });

        crate::sql_query("INSERT INTO kv (key, val) VALUES ('a', '1')")
            .execute(conn)
            .unwrap();
        crate::sql_query("UPDATE kv SET val = '2' WHERE key = 'a'")
            .execute(conn)
            .unwrap();
        crate::sql_query("DELETE FROM kv WHERE key = 'a'")
            .execute(conn)
            .unwrap();

        assert!(
            events.lock().unwrap().is_empty(),
            "update hook must not fire for WITHOUT ROWID tables"
        );
    }

    /// When a UNIQUE constraint conflict is resolved via ON CONFLICT REPLACE,
    /// the implicit deletion of the conflicting row does NOT fire the update
    /// hook. Only the INSERT for the new row fires.
    /// See: https://www.sqlite.org/c3ref/update_hook.html
    #[diesel_test_helper::test]
    fn update_hook_silent_for_on_conflict_replace_deletion() {
        use std::sync::{Arc, Mutex};
        let conn = &mut connection();

        crate::sql_query("CREATE TABLE uq (id INTEGER PRIMARY KEY, val TEXT NOT NULL UNIQUE)")
            .execute(conn)
            .unwrap();

        crate::sql_query("INSERT INTO uq (id, val) VALUES (1, 'original')")
            .execute(conn)
            .unwrap();

        let events: Arc<Mutex<Vec<(SqliteChangeOp, i64)>>> = Arc::new(Mutex::new(Vec::new()));
        let e2 = events.clone();

        conn.on_change(SqliteChangeOps::ALL, move |ev| {
            if ev.table_name == "uq" {
                e2.lock().unwrap().push((ev.op, ev.rowid));
            }
        });

        // INSERT OR REPLACE with a conflicting val: the old row (id=1) is
        // silently deleted by SQLite and the new row (id=2) is inserted.
        // The hook fires only for the INSERT of the new row.
        crate::sql_query("INSERT OR REPLACE INTO uq (id, val) VALUES (2, 'original')")
            .execute(conn)
            .unwrap();

        let recorded = events.lock().unwrap();
        assert_eq!(
            recorded.len(),
            1,
            "expected only 1 event (INSERT), got: {:?}",
            *recorded
        );
        assert_eq!(recorded[0].0, SqliteChangeOp::Insert);
        assert_eq!(recorded[0].1, 2, "new row should have rowid 2");
    }

    /// DELETE FROM without a WHERE clause triggers the truncate optimization,
    /// which bypasses the update hook entirely: no per-row DELETE events fire.
    /// See: https://www.sqlite.org/lang_delete.html#truncateopt
    #[diesel_test_helper::test]
    fn update_hook_silent_for_truncate_optimization() {
        use std::sync::{Arc, Mutex};
        let conn = &mut connection();

        // The truncate optimization applies when:
        //   1. No WHERE clause
        //   2. No RETURNING clause
        //   3. No triggers on the table
        crate::sql_query("CREATE TABLE bulk (id INTEGER PRIMARY KEY, data TEXT NOT NULL)")
            .execute(conn)
            .unwrap();

        crate::sql_query("INSERT INTO bulk (data) VALUES ('a'), ('b'), ('c')")
            .execute(conn)
            .unwrap();

        let events: Arc<Mutex<Vec<SqliteChangeOp>>> = Arc::new(Mutex::new(Vec::new()));
        let e2 = events.clone();

        conn.on_change(SqliteChangeOps::ALL, move |ev| {
            if ev.table_name == "bulk" {
                e2.lock().unwrap().push(ev.op);
            }
        });

        // DELETE without WHERE — truncate optimization kicks in.
        crate::sql_query("DELETE FROM bulk").execute(conn).unwrap();

        assert!(
            events.lock().unwrap().is_empty(),
            "truncate optimization should bypass the update hook"
        );
    }

    /// When a table has triggers, the truncate optimization is disabled, so
    /// DELETE without WHERE fires per-row DELETE events as normal.
    #[diesel_test_helper::test]
    fn update_hook_fires_for_delete_all_when_triggers_disable_truncate() {
        use std::sync::{Arc, Mutex};
        let conn = &mut connection();

        crate::sql_query("CREATE TABLE triggered (id INTEGER PRIMARY KEY, data TEXT NOT NULL)")
            .execute(conn)
            .unwrap();
        // A no-op trigger is enough to disable the truncate optimization.
        crate::sql_query(
            "CREATE TRIGGER trg_triggered BEFORE DELETE ON triggered \
             BEGIN SELECT 1; END",
        )
        .execute(conn)
        .unwrap();

        crate::sql_query("INSERT INTO triggered (data) VALUES ('x'), ('y'), ('z')")
            .execute(conn)
            .unwrap();

        let deletes: Arc<Mutex<Vec<i64>>> = Arc::new(Mutex::new(Vec::new()));
        let d2 = deletes.clone();

        conn.on_change(SqliteChangeOps::DELETE, move |ev| {
            if ev.table_name == "triggered" {
                d2.lock().unwrap().push(ev.rowid);
            }
        });

        // DELETE without WHERE, but triggers exist ⇒ no truncate optimization.
        crate::sql_query("DELETE FROM triggered")
            .execute(conn)
            .unwrap();

        assert_eq!(
            deletes.lock().unwrap().len(),
            3,
            "with triggers present, DELETE without WHERE fires per-row hooks"
        );
    }

    /// Modifications to internal system tables like sqlite_sequence
    /// (used by AUTOINCREMENT) do not trigger the update hook.
    /// See: https://www.sqlite.org/c3ref/update_hook.html
    #[diesel_test_helper::test]
    fn update_hook_silent_for_internal_sqlite_sequence() {
        use std::sync::{Arc, Mutex};
        let conn = &mut connection();

        // AUTOINCREMENT causes SQLite to maintain sqlite_sequence.
        crate::sql_query(
            "CREATE TABLE seq_test (id INTEGER PRIMARY KEY AUTOINCREMENT, name TEXT NOT NULL)",
        )
        .execute(conn)
        .unwrap();

        let tables: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));
        let t2 = tables.clone();

        conn.on_change(SqliteChangeOps::ALL, move |ev| {
            t2.lock().unwrap().push(ev.table_name.to_owned());
        });

        crate::sql_query("INSERT INTO seq_test (name) VALUES ('row1')")
            .execute(conn)
            .unwrap();

        let recorded = tables.lock().unwrap();
        // Only the user table should appear; sqlite_sequence must be absent.
        assert!(
            recorded.iter().all(|t| t == "seq_test"),
            "expected only 'seq_test' events, got: {:?}",
            *recorded
        );
        assert!(
            !recorded.iter().any(|t| t == "sqlite_sequence"),
            "sqlite_sequence modifications must not trigger the update hook"
        );
    }

    /// INSERT OR REPLACE on the primary key itself: when a row with the same
    /// PK already exists, the old row is silently deleted and the new row is
    /// inserted. The hook reports only the INSERT, not the implicit DELETE.
    #[diesel_test_helper::test]
    fn update_hook_silent_for_replace_into_on_pk_conflict() {
        use std::sync::{Arc, Mutex};
        let conn = &mut connection();

        crate::sql_query("CREATE TABLE rep (id INTEGER PRIMARY KEY, val TEXT NOT NULL)")
            .execute(conn)
            .unwrap();

        crate::sql_query("INSERT INTO rep (id, val) VALUES (1, 'old')")
            .execute(conn)
            .unwrap();

        let events: Arc<Mutex<Vec<(SqliteChangeOp, i64)>>> = Arc::new(Mutex::new(Vec::new()));
        let e2 = events.clone();

        conn.on_change(SqliteChangeOps::ALL, move |ev| {
            if ev.table_name == "rep" {
                e2.lock().unwrap().push((ev.op, ev.rowid));
            }
        });

        // REPLACE INTO with a conflicting PK.
        crate::sql_query("REPLACE INTO rep (id, val) VALUES (1, 'new')")
            .execute(conn)
            .unwrap();

        let recorded = events.lock().unwrap();
        // Only one INSERT event, no DELETE for the old row.
        assert_eq!(
            recorded.len(),
            1,
            "expected 1 event for REPLACE INTO, got: {:?}",
            *recorded
        );
        assert_eq!(recorded[0].0, SqliteChangeOp::Insert);
        assert_eq!(recorded[0].1, 1);
    }

    // ===================================================================
    // Commit / rollback hook tests
    // ===================================================================

    #[derive(crate::QueryableByName)]
    struct CountResult {
        #[diesel(sql_type = crate::sql_types::BigInt)]
        c: i64,
    }

    #[diesel_test_helper::test]
    fn on_commit_fires_on_commit() {
        use std::sync::{Arc, Mutex};
        let conn = &mut connection();

        let count = Arc::new(Mutex::new(0u32));
        let c2 = count.clone();

        conn.on_commit(move || {
            *c2.lock().unwrap() += 1;
            false // allow commit
        });

        conn.immediate_transaction(|conn| {
            crate::sql_query("CREATE TABLE t1 (id INTEGER PRIMARY KEY)")
                .execute(conn)
                .unwrap();
            Ok::<_, crate::result::Error>(())
        })
        .unwrap();

        assert_eq!(*count.lock().unwrap(), 1);
    }

    #[diesel_test_helper::test]
    fn on_commit_returning_true_forces_rollback() {
        let conn = &mut connection();

        crate::sql_query("CREATE TABLE t_commit (id INTEGER PRIMARY KEY)")
            .execute(conn)
            .unwrap();

        conn.on_commit(|| true /* convert commit to rollback */);

        // The transaction will attempt to commit, but the hook will convert
        // it to a rollback. diesel's AnsiTransactionManager will see the
        // failure from the COMMIT statement (sqlite returns an error when
        // the commit hook returns non-zero and the commit is aborted).
        let result = conn.immediate_transaction(|conn| {
            crate::sql_query("INSERT INTO t_commit (id) VALUES (1)")
                .execute(conn)
                .unwrap();
            Ok::<_, crate::result::Error>(())
        });

        // The transaction should have been rolled back.
        assert!(result.is_err());

        // Remove the hook so subsequent queries don't fail.
        conn.remove_commit_hook();

        // Verify the row was not persisted.
        let cnt: i64 = crate::sql_query("SELECT COUNT(*) as c FROM t_commit")
            .get_result::<CountResult>(conn)
            .unwrap()
            .c;
        assert_eq!(cnt, 0);
    }

    #[diesel_test_helper::test]
    fn on_rollback_fires_on_explicit_rollback() {
        use std::sync::{Arc, Mutex};
        let conn = &mut connection();

        crate::sql_query("CREATE TABLE t_rb (id INTEGER PRIMARY KEY)")
            .execute(conn)
            .unwrap();

        let count = Arc::new(Mutex::new(0u32));
        let c2 = count.clone();

        conn.on_rollback(move || {
            *c2.lock().unwrap() += 1;
        });

        // Force a rollback by returning Err from the transaction closure.
        let _ = conn.immediate_transaction(|conn| {
            crate::sql_query("INSERT INTO t_rb (id) VALUES (1)")
                .execute(conn)
                .unwrap();
            Err::<(), _>(crate::result::Error::RollbackTransaction)
        });

        assert_eq!(*count.lock().unwrap(), 1);
    }

    #[diesel_test_helper::test]
    fn on_rollback_fires_when_commit_hook_forces_rollback() {
        use std::sync::{Arc, Mutex};
        let conn = &mut connection();

        crate::sql_query("CREATE TABLE t_rb2 (id INTEGER PRIMARY KEY)")
            .execute(conn)
            .unwrap();

        let rb_count = Arc::new(Mutex::new(0u32));
        let rb2 = rb_count.clone();

        conn.on_commit(|| true /* force rollback */);
        conn.on_rollback(move || {
            *rb2.lock().unwrap() += 1;
        });

        let _ = conn.immediate_transaction(|conn| {
            crate::sql_query("INSERT INTO t_rb2 (id) VALUES (1)")
                .execute(conn)
                .unwrap();
            Ok::<_, crate::result::Error>(())
        });

        // Rollback hook should have fired.
        assert_eq!(*rb_count.lock().unwrap(), 1);

        conn.remove_commit_hook();
        conn.remove_rollback_hook();
    }

    #[diesel_test_helper::test]
    fn on_rollback_does_not_fire_on_connection_close() {
        use std::sync::{Arc, Mutex};

        let count = Arc::new(Mutex::new(0u32));
        let c2 = count.clone();

        {
            let conn = &mut connection();
            conn.on_rollback(move || {
                *c2.lock().unwrap() += 1;
            });
            // conn is dropped here — implicit close, not a rollback.
        }

        assert_eq!(*count.lock().unwrap(), 0);
    }

    #[diesel_test_helper::test]
    fn replacing_commit_hook_drops_old() {
        use std::sync::{Arc, Mutex};
        let conn = &mut connection();

        let old_count = Arc::new(Mutex::new(0u32));
        let new_count = Arc::new(Mutex::new(0u32));
        let oc = old_count.clone();
        let nc = new_count.clone();

        conn.on_commit(move || {
            *oc.lock().unwrap() += 1;
            false
        });

        // Replace with a new hook.
        conn.on_commit(move || {
            *nc.lock().unwrap() += 1;
            false
        });

        conn.immediate_transaction(|conn| {
            crate::sql_query("CREATE TABLE t_replace (id INTEGER PRIMARY KEY)")
                .execute(conn)
                .unwrap();
            Ok::<_, crate::result::Error>(())
        })
        .unwrap();

        assert_eq!(*old_count.lock().unwrap(), 0);
        assert_eq!(*new_count.lock().unwrap(), 1);
    }

    #[diesel_test_helper::test]
    fn remove_commit_hook_disables_callback() {
        use std::sync::{Arc, Mutex};
        let conn = &mut connection();

        let count = Arc::new(Mutex::new(0u32));
        let c2 = count.clone();

        conn.on_commit(move || {
            *c2.lock().unwrap() += 1;
            false
        });

        conn.remove_commit_hook();

        conn.immediate_transaction(|conn| {
            crate::sql_query("CREATE TABLE t_rem (id INTEGER PRIMARY KEY)")
                .execute(conn)
                .unwrap();
            Ok::<_, crate::result::Error>(())
        })
        .unwrap();

        assert_eq!(*count.lock().unwrap(), 0);
    }

    #[diesel_test_helper::test]
    fn remove_rollback_hook_disables_callback() {
        use std::sync::{Arc, Mutex};
        let conn = &mut connection();

        crate::sql_query("CREATE TABLE t_rem_rb (id INTEGER PRIMARY KEY)")
            .execute(conn)
            .unwrap();

        let count = Arc::new(Mutex::new(0u32));
        let c2 = count.clone();

        conn.on_rollback(move || {
            *c2.lock().unwrap() += 1;
        });

        conn.remove_rollback_hook();

        let _ = conn.immediate_transaction(|conn| {
            crate::sql_query("INSERT INTO t_rem_rb (id) VALUES (1)")
                .execute(conn)
                .unwrap();
            Err::<(), _>(crate::result::Error::RollbackTransaction)
        });

        assert_eq!(*count.lock().unwrap(), 0);
    }

    // ---------------------------------------------------------------
    // WAL hook tests
    //
    // Gated out on WASM because these tests need a file-backed database
    // (WAL mode does not work with `:memory:`), and `tempfile::tempdir()`
    // panics on WASM due to the lack of a filesystem.
    //
    // The WAL *API* itself (`on_wal`, `remove_wal_hook`) is available on
    // all platforms, including WASM. In principle WAL could work on WASM
    // with the `sahpool` VFS (OPFS-backed), but that VFS is only usable
    // inside a Dedicated Worker context. The default `memory` VFS has no
    // persistent storage, and `relaxed-idb` (IndexedDB) does not provide
    // the file-level I/O (WAL/SHM sidecar files) that WAL mode requires.
    // Additionally, `sqlite-wasm-rs` compiles SQLite with
    // `-DSQLITE_THREADSAFE=0` and none of its VFS backends support
    // multiple connections, further limiting WAL's utility on WASM.
    //
    // Testing WAL on specific WASM VFS backends is better suited for
    // downstream integration tests.
    // ---------------------------------------------------------------

    #[cfg(not(all(target_family = "wasm", target_os = "unknown")))]
    /// Helper: create a file-backed connection (WAL requires a real file).
    fn wal_connection() -> (SqliteConnection, tempfile::TempDir) {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.db");
        let conn = SqliteConnection::establish(path.to_str().unwrap()).unwrap();
        (conn, dir)
    }

    #[cfg(not(all(target_family = "wasm", target_os = "unknown")))]
    #[diesel_test_helper::test]
    fn on_wal_fires_in_wal_mode() {
        use std::sync::{Arc, Mutex};
        let (conn, _dir) = &mut wal_connection();

        // Enable WAL mode.
        crate::sql_query("PRAGMA journal_mode=WAL")
            .execute(conn)
            .unwrap();

        crate::sql_query("CREATE TABLE t_wal (id INTEGER PRIMARY KEY)")
            .execute(conn)
            .unwrap();

        let calls: Arc<Mutex<Vec<(String, i32)>>> = Arc::new(Mutex::new(Vec::new()));
        let c2 = calls.clone();

        conn.on_wal(move |db_name, n_pages| {
            c2.lock().unwrap().push((db_name.to_owned(), n_pages));
        });

        crate::sql_query("INSERT INTO t_wal (id) VALUES (1)")
            .execute(conn)
            .unwrap();

        let recorded = calls.lock().unwrap();
        assert!(
            !recorded.is_empty(),
            "WAL hook should have fired at least once"
        );
        assert_eq!(recorded.last().unwrap().0, "main");
        assert!(recorded.last().unwrap().1 > 0, "n_pages should be positive");
    }

    #[cfg(not(all(target_family = "wasm", target_os = "unknown")))]
    #[diesel_test_helper::test]
    fn replacing_wal_hook_drops_old() {
        use std::sync::{Arc, Mutex};
        let (conn, _dir) = &mut wal_connection();

        crate::sql_query("PRAGMA journal_mode=WAL")
            .execute(conn)
            .unwrap();
        crate::sql_query("CREATE TABLE t_wal2 (id INTEGER PRIMARY KEY)")
            .execute(conn)
            .unwrap();

        let old_count = Arc::new(Mutex::new(0u32));
        let new_count = Arc::new(Mutex::new(0u32));

        let c_old = old_count.clone();
        conn.on_wal(move |_, _| {
            *c_old.lock().unwrap() += 1;
        });

        // Replace with a new hook.
        let c_new = new_count.clone();
        conn.on_wal(move |_, _| {
            *c_new.lock().unwrap() += 1;
        });

        crate::sql_query("INSERT INTO t_wal2 (id) VALUES (1)")
            .execute(conn)
            .unwrap();

        // Old hook should NOT have fired after replacement.
        let old_before = *old_count.lock().unwrap();
        crate::sql_query("INSERT INTO t_wal2 (id) VALUES (2)")
            .execute(conn)
            .unwrap();
        assert_eq!(
            *old_count.lock().unwrap(),
            old_before,
            "old WAL hook should not fire after replacement"
        );
        assert!(*new_count.lock().unwrap() > 0, "new WAL hook should fire");
    }

    #[cfg(not(all(target_family = "wasm", target_os = "unknown")))]
    #[diesel_test_helper::test]
    fn remove_wal_hook_disables_callback() {
        use std::sync::{Arc, Mutex};
        let (conn, _dir) = &mut wal_connection();

        crate::sql_query("PRAGMA journal_mode=WAL")
            .execute(conn)
            .unwrap();
        crate::sql_query("CREATE TABLE t_wal3 (id INTEGER PRIMARY KEY)")
            .execute(conn)
            .unwrap();

        let count = Arc::new(Mutex::new(0u32));
        let c2 = count.clone();

        conn.on_wal(move |_, _| {
            *c2.lock().unwrap() += 1;
        });

        conn.remove_wal_hook();

        crate::sql_query("INSERT INTO t_wal3 (id) VALUES (1)")
            .execute(conn)
            .unwrap();

        assert_eq!(*count.lock().unwrap(), 0);
    }

    #[cfg(not(all(target_family = "wasm", target_os = "unknown")))]
    #[diesel_test_helper::test]
    fn wal_hook_does_not_fire_in_default_journal_mode() {
        use std::sync::{Arc, Mutex};
        let (conn, _dir) = &mut wal_connection();

        // Default journal mode for file-based databases is "delete" (not WAL).
        crate::sql_query("CREATE TABLE t_wal4 (id INTEGER PRIMARY KEY)")
            .execute(conn)
            .unwrap();

        let count = Arc::new(Mutex::new(0u32));
        let c2 = count.clone();

        conn.on_wal(move |_, _| {
            *c2.lock().unwrap() += 1;
        });

        crate::sql_query("INSERT INTO t_wal4 (id) VALUES (1)")
            .execute(conn)
            .unwrap();

        assert_eq!(
            *count.lock().unwrap(),
            0,
            "WAL hook should not fire when not in WAL mode"
        );
    }
}
