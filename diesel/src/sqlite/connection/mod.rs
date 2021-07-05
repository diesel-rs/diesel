extern crate libsqlite3_sys as ffi;

mod functions;
#[doc(hidden)]
pub mod raw;
mod row;
mod serialized_value;
mod sqlite_value;
mod statement_iterator;
mod stmt;

pub use self::sqlite_value::SqliteValue;

use std::os::raw as libc;

use self::raw::RawConnection;
use self::statement_iterator::*;
use self::stmt::{Statement, StatementUse};
use super::SqliteAggregateFunction;
use crate::connection::*;
use crate::deserialize::{FromSqlRow, StaticallySizedRow};
use crate::expression::QueryMetadata;
use crate::query_builder::bind_collector::RawBytesBindCollector;
use crate::query_builder::*;
use crate::result::*;
use crate::serialize::ToSql;
use crate::sql_types::HasSqlType;
use crate::sqlite::Sqlite;

/// Connections for the SQLite backend. Unlike other backends, SQLite supported
/// connection URLs are:
///
/// - File paths (`test.db`)
/// - [URIs](https://sqlite.org/uri.html) (`file://test.db`)
/// - Special identifiers (`:memory:`)
#[allow(missing_debug_implementations)]
pub struct SqliteConnection {
    // statement_cache needs to be before raw_connection
    // otherwise we will get errors about open statements before closing the
    // connection itself
    statement_cache: StatementCache<Sqlite, Statement>,
    raw_connection: RawConnection,
    transaction_state: AnsiTransactionManager,
}

// This relies on the invariant that RawConnection or Statement are never
// leaked. If a reference to one of those was held on a different thread, this
// would not be thread safe.
unsafe impl Send for SqliteConnection {}

impl SimpleConnection for SqliteConnection {
    fn batch_execute(&mut self, query: &str) -> QueryResult<()> {
        self.raw_connection.exec(query)
    }
}

impl<'a> IterableConnection<'a, Sqlite> for SqliteConnection {
    type Cursor = StatementIterator<'a>;
    type Row = self::row::SqliteRow<'a>;
}

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
        use crate::result::ConnectionError::CouldntSetupConfiguration;

        let raw_connection = RawConnection::establish(database_url)?;
        let conn = Self {
            statement_cache: StatementCache::new(),
            raw_connection,
            transaction_state: AnsiTransactionManager::default(),
        };
        conn.register_diesel_sql_functions()
            .map_err(CouldntSetupConfiguration)?;
        Ok(conn)
    }

    #[doc(hidden)]
    fn execute(&mut self, query: &str) -> QueryResult<usize> {
        self.batch_execute(query)?;
        Ok(self.raw_connection.rows_affected_by_last_query())
    }

    #[doc(hidden)]
    fn load<'a, T>(
        &'a mut self,
        source: T,
    ) -> QueryResult<<Self as IterableConnection<'a, Self::Backend>>::Cursor>
    where
        T: AsQuery,
        T::Query: QueryFragment<Self::Backend> + QueryId,
        Self::Backend: QueryMetadata<T::SqlType>,
    {
        let stmt = self.prepared_query(&source.as_query())?;

        let statement_use = StatementUse::new(stmt);
        Ok(StatementIterator::new(statement_use))
    }

    #[doc(hidden)]
    fn execute_returning_count<T>(&mut self, source: &T) -> QueryResult<usize>
    where
        T: QueryFragment<Self::Backend> + QueryId,
    {
        let stmt = self.prepared_query(source)?;

        let statement_use = StatementUse::new(stmt);
        statement_use.run()?;

        Ok(self.raw_connection.rows_affected_by_last_query())
    }

    fn transaction_state(&mut self) -> &mut AnsiTransactionManager
    where
        Self: Sized,
    {
        &mut self.transaction_state
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

    fn prepared_query<'a, T: QueryFragment<Sqlite> + QueryId>(
        &'a mut self,
        source: &'_ T,
    ) -> QueryResult<MaybeCached<'a, Statement>> {
        let raw_connection = &self.raw_connection;
        let cache = &mut self.statement_cache;
        let mut statement = cache.cached_statement(source, &[], |sql, is_cached| {
            Statement::prepare(raw_connection, sql, is_cached)
        })?;

        let mut bind_collector = RawBytesBindCollector::<Sqlite>::new();
        source.collect_binds(&mut bind_collector, &mut ())?;
        let metadata = bind_collector.metadata;
        let binds = bind_collector.binds;
        for (tpe, value) in metadata.into_iter().zip(binds) {
            statement.bind(tpe, value)?;
        }

        Ok(statement)
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
    sql_function!(fn fun_case(x: Text) -> Text);

    #[test]
    fn register_custom_function() {
        let connection = &mut SqliteConnection::establish(":memory:").unwrap();
        fun_case::register_impl(connection, |x: String| {
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

    sql_function!(fn my_add(x: Integer, y: Integer) -> Integer);

    #[test]
    fn register_multiarg_function() {
        let connection = &mut SqliteConnection::establish(":memory:").unwrap();
        my_add::register_impl(connection, |x: i32, y: i32| x + y).unwrap();

        let added = crate::select(my_add(1, 2)).get_result::<i32>(connection);
        assert_eq!(Ok(3), added);
    }

    sql_function!(fn answer() -> Integer);

    #[test]
    fn register_noarg_function() {
        let connection = &mut SqliteConnection::establish(":memory:").unwrap();
        answer::register_impl(&connection, || 42).unwrap();

        let answer = crate::select(answer()).get_result::<i32>(connection);
        assert_eq!(Ok(42), answer);
    }

    #[test]
    fn register_nondeterministic_noarg_function() {
        let connection = &mut SqliteConnection::establish(":memory:").unwrap();
        answer::register_nondeterministic_impl(&connection, || 42).unwrap();

        let answer = crate::select(answer()).get_result::<i32>(connection);
        assert_eq!(Ok(42), answer);
    }

    sql_function!(fn add_counter(x: Integer) -> Integer);

    #[test]
    fn register_nondeterministic_function() {
        let connection = &mut SqliteConnection::establish(":memory:").unwrap();
        let mut y = 0;
        add_counter::register_nondeterministic_impl(connection, move |x: i32| {
            y += 1;
            x + y
        })
        .unwrap();

        let added = crate::select((add_counter(1), add_counter(1), add_counter(1)))
            .get_result::<(i32, i32, i32)>(connection);
        assert_eq!(Ok((2, 3, 4)), added);
    }

    use crate::sqlite::SqliteAggregateFunction;

    sql_function! {
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
        connection
            .execute(
                "CREATE TABLE my_sum_example (id integer primary key autoincrement, value integer)",
            )
            .unwrap();
        connection
            .execute("INSERT INTO my_sum_example (value) VALUES (1), (2), (3)")
            .unwrap();

        my_sum::register_impl::<MySum, _>(connection).unwrap();

        let result = my_sum_example
            .select(my_sum(value))
            .get_result::<i32>(connection);
        assert_eq!(Ok(6), result);
    }

    #[test]
    fn register_aggregate_function_returns_finalize_default_on_empty_set() {
        use self::my_sum_example::dsl::*;

        let connection = &mut SqliteConnection::establish(":memory:").unwrap();
        connection
            .execute(
                "CREATE TABLE my_sum_example (id integer primary key autoincrement, value integer)",
            )
            .unwrap();

        my_sum::register_impl::<MySum, _>(connection).unwrap();

        let result = my_sum_example
            .select(my_sum(value))
            .get_result::<i32>(connection);
        assert_eq!(Ok(0), result);
    }

    sql_function! {
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
        connection
            .execute(
                r#"CREATE TABLE range_max_example (
                id integer primary key autoincrement,
                value1 integer,
                value2 integer,
                value3 integer
            )"#,
            )
            .unwrap();
        connection.execute("INSERT INTO range_max_example (value1, value2, value3) VALUES (3, 2, 1), (2, 2, 2)").unwrap();

        range_max::register_impl::<RangeMax<i32>, _, _, _>(connection).unwrap();
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

        connection
            .execute(
                "CREATE TABLE my_collation_example (id integer primary key autoincrement, value text collate RUSTNOCASE)",
            )
            .unwrap();
        connection
            .execute("INSERT INTO my_collation_example (value) VALUES ('foo'), ('FOo'), ('f00')")
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
}
