extern crate libsqlite3_sys as ffi;

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

use std::os::raw as libc;

use self::raw::RawConnection;
use self::statement_iterator::*;
use self::stmt::{Statement, StatementUse};
use super::SqliteAggregateFunction;
use crate::connection::instrumentation::StrQueryHelper;
use crate::connection::statement_cache::StatementCache;
use crate::connection::*;
use crate::deserialize::{FromSqlRow, StaticallySizedRow};
use crate::expression::QueryMetadata;
use crate::query_builder::*;
use crate::result::*;
use crate::serialize::ToSql;
use crate::sql_types::{HasSqlType, TypeMetadata};
use crate::sqlite::Sqlite;

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
/// As `SqliteConnection` only supports a single loading mode implementation
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
/// { // scope to restrict the lifetime of the iterator
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
#[allow(missing_debug_implementations)]
#[cfg(feature = "sqlite")]
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
    instrumentation: Option<Box<dyn Instrumentation>>,
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
    fn establish(database_url: &str) -> ConnectionResult<Self> {
        let mut instrumentation = crate::connection::instrumentation::get_default_instrumentation();
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
        statement_use
            .run()
            .map(|_| self.raw_connection.rows_affected_by_last_query())
    }

    fn transaction_state(&mut self) -> &mut AnsiTransactionManager
    where
        Self: Sized,
    {
        &mut self.transaction_state
    }

    fn instrumentation(&mut self) -> &mut dyn Instrumentation {
        &mut self.instrumentation
    }

    fn set_instrumentation(&mut self, instrumentation: impl Instrumentation) {
        self.instrumentation = Some(Box::new(instrumentation));
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
    ) -> &mut (dyn std::any::Any + 'a) {
        lookup
    }

    fn from_any(
        lookup: &mut dyn std::any::Any,
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
            |sql, is_cached| Statement::prepare(raw_connection, sql, is_cached),
            &mut self.instrumentation,
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

        StatementUse::bind(statement, source, &mut self.instrumentation)
    }

    #[doc(hidden)]
    pub fn register_sql_function<ArgsSqlType, RetSqlType, Args, Ret, F>(
        &mut self,
        fn_name: &str,
        deterministic: bool,
        mut f: F,
    ) -> QueryResult<()>
    where
        F: FnMut(Args) -> Ret + std::panic::UnwindSafe + Send + 'static,
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
        F: FnMut() -> Ret + std::panic::UnwindSafe + Send + 'static,
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
        A: SqliteAggregateFunction<Args, Output = Ret> + 'static + Send + std::panic::UnwindSafe,
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
        F: Fn(&str, &str) -> std::cmp::Ordering + Send + 'static + std::panic::UnwindSafe,
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

    fn register_diesel_sql_functions(&self) -> QueryResult<()> {
        use crate::sql_types::{Integer, Text};

        functions::register::<Text, Integer, _, _, _>(
            &self.raw_connection,
            "diesel_manage_updated_at",
            false,
            |conn, table_name: String| {
                conn.exec(&format!(
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
            instrumentation: None,
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
    use crate::sql_types::Integer;

    #[test]
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

        let connection = &mut SqliteConnection::establish(":memory:").unwrap();
        let _ =
            crate::sql_query("CREATE TABLE users (id INTEGER PRIMARY KEY, name TEXT, email TEXT)")
                .execute(connection);
        let _ = crate::sql_query("INSERT INTO users (name, email) VALUES ('John Doe', 'john.doe@example.com'), ('Jane Doe', 'jane.doe@example.com')")
            .execute(connection);

        let serialized_database = connection.serialize_database_to_buffer();

        let connection = &mut SqliteConnection::establish(":memory:").unwrap();
        connection
            .deserialize_readonly_database_from_buffer(serialized_database.as_slice())
            .unwrap();

        let query = sql::<(Integer, Text, Text)>("SELECT id, name, email FROM users ORDER BY id");
        let actual_users = query.load::<(i32, String, String)>(connection).unwrap();

        assert_eq!(expected_users, actual_users);
    }

    #[test]
    fn prepared_statements_are_cached_when_run() {
        let connection = &mut SqliteConnection::establish(":memory:").unwrap();
        let query = crate::select(1.into_sql::<Integer>());

        assert_eq!(Ok(1), query.get_result(connection));
        assert_eq!(Ok(1), query.get_result(connection));
        assert_eq!(1, connection.statement_cache.len());
    }

    #[test]
    fn sql_literal_nodes_are_not_cached() {
        let connection = &mut SqliteConnection::establish(":memory:").unwrap();
        let query = crate::select(sql::<Integer>("1"));

        assert_eq!(Ok(1), query.get_result(connection));
        assert_eq!(0, connection.statement_cache.len());
    }

    #[test]
    fn queries_containing_sql_literal_nodes_are_not_cached() {
        let connection = &mut SqliteConnection::establish(":memory:").unwrap();
        let one_as_expr = 1.into_sql::<Integer>();
        let query = crate::select(one_as_expr.eq(sql::<Integer>("1")));

        assert_eq!(Ok(true), query.get_result(connection));
        assert_eq!(0, connection.statement_cache.len());
    }

    #[test]
    fn queries_containing_in_with_vec_are_not_cached() {
        let connection = &mut SqliteConnection::establish(":memory:").unwrap();
        let one_as_expr = 1.into_sql::<Integer>();
        let query = crate::select(one_as_expr.eq_any(vec![1, 2, 3]));

        assert_eq!(Ok(true), query.get_result(connection));
        assert_eq!(0, connection.statement_cache.len());
    }

    #[test]
    fn queries_containing_in_with_subselect_are_cached() {
        let connection = &mut SqliteConnection::establish(":memory:").unwrap();
        let one_as_expr = 1.into_sql::<Integer>();
        let query = crate::select(one_as_expr.eq_any(crate::select(one_as_expr)));

        assert_eq!(Ok(true), query.get_result(connection));
        assert_eq!(1, connection.statement_cache.len());
    }

    use crate::sql_types::Text;
    define_sql_function!(fn fun_case(x: Text) -> Text);

    #[test]
    fn register_custom_function() {
        let connection = &mut SqliteConnection::establish(":memory:").unwrap();
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

    define_sql_function!(fn my_add(x: Integer, y: Integer) -> Integer);

    #[test]
    fn register_multiarg_function() {
        let connection = &mut SqliteConnection::establish(":memory:").unwrap();
        my_add_utils::register_impl(connection, |x: i32, y: i32| x + y).unwrap();

        let added = crate::select(my_add(1, 2)).get_result::<i32>(connection);
        assert_eq!(Ok(3), added);
    }

    define_sql_function!(fn answer() -> Integer);

    #[test]
    fn register_noarg_function() {
        let connection = &mut SqliteConnection::establish(":memory:").unwrap();
        answer_utils::register_impl(connection, || 42).unwrap();

        let answer = crate::select(answer()).get_result::<i32>(connection);
        assert_eq!(Ok(42), answer);
    }

    #[test]
    fn register_nondeterministic_noarg_function() {
        let connection = &mut SqliteConnection::establish(":memory:").unwrap();
        answer_utils::register_nondeterministic_impl(connection, || 42).unwrap();

        let answer = crate::select(answer()).get_result::<i32>(connection);
        assert_eq!(Ok(42), answer);
    }

    define_sql_function!(fn add_counter(x: Integer) -> Integer);

    #[test]
    fn register_nondeterministic_function() {
        let connection = &mut SqliteConnection::establish(":memory:").unwrap();
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

    define_sql_function! {
        #[aggregate]
        fn my_sum(expr: Integer) -> Integer;
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

    #[test]
    fn register_aggregate_function() {
        use self::my_sum_example::dsl::*;

        let connection = &mut SqliteConnection::establish(":memory:").unwrap();
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

    #[test]
    fn register_aggregate_function_returns_finalize_default_on_empty_set() {
        use self::my_sum_example::dsl::*;

        let connection = &mut SqliteConnection::establish(":memory:").unwrap();
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

    define_sql_function! {
        #[aggregate]
        fn range_max(expr1: Integer, expr2: Integer, expr3: Integer) -> Nullable<Integer>;
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

    #[test]
    fn register_aggregate_multiarg_function() {
        use self::range_max_example::dsl::*;

        let connection = &mut SqliteConnection::establish(":memory:").unwrap();
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

    #[test]
    fn register_collation_function() {
        use self::my_collation_example::dsl::*;

        let connection = &mut SqliteConnection::establish(":memory:").unwrap();

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
    #[test]
    fn test_correct_seralization_of_owned_strings() {
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

        let connection = &mut SqliteConnection::establish(":memory:").unwrap();

        let res = crate::select(
            CustomWrapper("".into())
                .into_sql::<crate::sql_types::Text>()
                .nullable(),
        )
        .get_result::<Option<String>>(connection)
        .unwrap();
        assert_eq!(res, Some(String::new()));
    }

    #[test]
    fn test_correct_seralization_of_owned_bytes() {
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

        let connection = &mut SqliteConnection::establish(":memory:").unwrap();

        let res = crate::select(
            CustomWrapper(Vec::new())
                .into_sql::<crate::sql_types::Binary>()
                .nullable(),
        )
        .get_result::<Option<Vec<u8>>>(connection)
        .unwrap();
        assert_eq!(res, Some(Vec::new()));
    }
}
