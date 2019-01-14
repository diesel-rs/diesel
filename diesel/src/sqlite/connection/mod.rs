extern crate libsqlite3_sys as ffi;

mod functions;
#[doc(hidden)]
pub mod raw;
mod serialized_value;
mod sqlite_value;
mod statement_iterator;
mod stmt;

pub use self::sqlite_value::SqliteValue;

use std::os::raw as libc;

use self::raw::RawConnection;
use self::statement_iterator::*;
use self::stmt::{Statement, StatementUse};
use connection::*;
use deserialize::{Queryable, QueryableByName};
use query_builder::bind_collector::RawBytesBindCollector;
use query_builder::*;
use result::*;
use serialize::ToSql;
use sql_types::HasSqlType;
use sqlite::Sqlite;

/// Connections for the SQLite backend. Unlike other backends, "connection URLs"
/// for SQLite are file paths, [URIs](https://sqlite.org/uri.html), or special
/// identifiers like `:memory:`.
#[allow(missing_debug_implementations)]
pub struct SqliteConnection {
    statement_cache: StatementCache<Sqlite, Statement>,
    raw_connection: RawConnection,
    transaction_manager: AnsiTransactionManager,
}

// This relies on the invariant that RawConnection or Statement are never
// leaked. If a reference to one of those was held on a different thread, this
// would not be thread safe.
unsafe impl Send for SqliteConnection {}

impl SimpleConnection for SqliteConnection {
    fn batch_execute(&self, query: &str) -> QueryResult<()> {
        self.raw_connection.exec(query)
    }
}

impl Connection for SqliteConnection {
    type Backend = Sqlite;
    type TransactionManager = AnsiTransactionManager;

    fn establish(database_url: &str) -> ConnectionResult<Self> {
        use result::ConnectionError::CouldntSetupConfiguration;

        let raw_connection = RawConnection::establish(database_url)?;
        let conn = Self {
            statement_cache: StatementCache::new(),
            raw_connection,
            transaction_manager: AnsiTransactionManager::new(),
        };
        conn.register_diesel_sql_functions()
            .map_err(CouldntSetupConfiguration)?;
        Ok(conn)
    }

    #[doc(hidden)]
    fn execute(&self, query: &str) -> QueryResult<usize> {
        self.batch_execute(query)?;
        Ok(self.raw_connection.rows_affected_by_last_query())
    }

    #[doc(hidden)]
    fn query_by_index<T, U>(&self, source: T) -> QueryResult<Vec<U>>
    where
        T: AsQuery,
        T::Query: QueryFragment<Self::Backend> + QueryId,
        Self::Backend: HasSqlType<T::SqlType>,
        U: Queryable<T::SqlType, Self::Backend>,
    {
        let mut statement = self.prepare_query(&source.as_query())?;
        let statement_use = StatementUse::new(&mut statement);
        let iter = StatementIterator::new(statement_use);
        iter.collect()
    }

    #[doc(hidden)]
    fn query_by_name<T, U>(&self, source: &T) -> QueryResult<Vec<U>>
    where
        T: QueryFragment<Self::Backend> + QueryId,
        U: QueryableByName<Self::Backend>,
    {
        let mut statement = self.prepare_query(source)?;
        let statement_use = StatementUse::new(&mut statement);
        let iter = NamedStatementIterator::new(statement_use)?;
        iter.collect()
    }

    #[doc(hidden)]
    fn execute_returning_count<T>(&self, source: &T) -> QueryResult<usize>
    where
        T: QueryFragment<Self::Backend> + QueryId,
    {
        let mut statement = self.prepare_query(source)?;
        let mut statement_use = StatementUse::new(&mut statement);
        statement_use.run()?;
        Ok(self.raw_connection.rows_affected_by_last_query())
    }

    #[doc(hidden)]
    fn transaction_manager(&self) -> &Self::TransactionManager {
        &self.transaction_manager
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
    /// # #[macro_use] extern crate diesel;
    /// # include!("../../doctest_setup.rs");
    /// #
    /// # fn main() {
    /// #     run_test().unwrap();
    /// # }
    /// #
    /// # fn run_test() -> QueryResult<()> {
    /// #     let conn = SqliteConnection::establish(":memory:").unwrap();
    /// conn.immediate_transaction(|| {
    ///     // Do stuff in a transaction
    ///     Ok(())
    /// })
    /// # }
    /// ```
    pub fn immediate_transaction<T, E, F>(&self, f: F) -> Result<T, E>
    where
        F: FnOnce() -> Result<T, E>,
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
    /// # #[macro_use] extern crate diesel;
    /// # include!("../../doctest_setup.rs");
    /// #
    /// # fn main() {
    /// #     run_test().unwrap();
    /// # }
    /// #
    /// # fn run_test() -> QueryResult<()> {
    /// #     let conn = SqliteConnection::establish(":memory:").unwrap();
    /// conn.exclusive_transaction(|| {
    ///     // Do stuff in a transaction
    ///     Ok(())
    /// })
    /// # }
    /// ```
    pub fn exclusive_transaction<T, E, F>(&self, f: F) -> Result<T, E>
    where
        F: FnOnce() -> Result<T, E>,
        E: From<Error>,
    {
        self.transaction_sql(f, "BEGIN EXCLUSIVE")
    }

    fn transaction_sql<T, E, F>(&self, f: F, sql: &str) -> Result<T, E>
    where
        F: FnOnce() -> Result<T, E>,
        E: From<Error>,
    {
        let transaction_manager = self.transaction_manager();

        transaction_manager.begin_transaction_sql(self, sql)?;
        match f() {
            Ok(value) => {
                transaction_manager.commit_transaction(self)?;
                Ok(value)
            }
            Err(e) => {
                transaction_manager.rollback_transaction(self)?;
                Err(e)
            }
        }
    }

    fn prepare_query<T: QueryFragment<Sqlite> + QueryId>(
        &self,
        source: &T,
    ) -> QueryResult<MaybeCached<Statement>> {
        let mut statement = self.cached_prepared_statement(source)?;

        let mut bind_collector = RawBytesBindCollector::<Sqlite>::new();
        source.collect_binds(&mut bind_collector, &())?;
        let metadata = bind_collector.metadata;
        let binds = bind_collector.binds;
        for (tpe, value) in metadata.into_iter().zip(binds) {
            statement.bind(tpe, value)?;
        }

        Ok(statement)
    }

    fn cached_prepared_statement<T: QueryFragment<Sqlite> + QueryId>(
        &self,
        source: &T,
    ) -> QueryResult<MaybeCached<Statement>> {
        self.statement_cache.cached_statement(source, &[], |sql| {
            Statement::prepare(&self.raw_connection, sql)
        })
    }

    #[doc(hidden)]
    pub fn register_sql_function<ArgsSqlType, RetSqlType, Args, Ret, F>(
        &self,
        fn_name: &str,
        deterministic: bool,
        mut f: F,
    ) -> QueryResult<()>
    where
        F: FnMut(Args) -> Ret + Send + 'static,
        Args: Queryable<ArgsSqlType, Sqlite>,
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

    fn register_diesel_sql_functions(&self) -> QueryResult<()> {
        use sql_types::{Integer, Text};

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
    use dsl::sql;
    use prelude::*;
    use sql_types::Integer;

    #[test]
    fn prepared_statements_are_cached_when_run() {
        let connection = SqliteConnection::establish(":memory:").unwrap();
        let query = ::select(1.into_sql::<Integer>());

        assert_eq!(Ok(1), query.get_result(&connection));
        assert_eq!(Ok(1), query.get_result(&connection));
        assert_eq!(1, connection.statement_cache.len());
    }

    #[test]
    fn sql_literal_nodes_are_not_cached() {
        let connection = SqliteConnection::establish(":memory:").unwrap();
        let query = ::select(sql::<Integer>("1"));

        assert_eq!(Ok(1), query.get_result(&connection));
        assert_eq!(0, connection.statement_cache.len());
    }

    #[test]
    fn queries_containing_sql_literal_nodes_are_not_cached() {
        let connection = SqliteConnection::establish(":memory:").unwrap();
        let one_as_expr = 1.into_sql::<Integer>();
        let query = ::select(one_as_expr.eq(sql::<Integer>("1")));

        assert_eq!(Ok(true), query.get_result(&connection));
        assert_eq!(0, connection.statement_cache.len());
    }

    #[test]
    fn queries_containing_in_with_vec_are_not_cached() {
        let connection = SqliteConnection::establish(":memory:").unwrap();
        let one_as_expr = 1.into_sql::<Integer>();
        let query = ::select(one_as_expr.eq_any(vec![1, 2, 3]));

        assert_eq!(Ok(true), query.get_result(&connection));
        assert_eq!(0, connection.statement_cache.len());
    }

    #[test]
    fn queries_containing_in_with_subselect_are_cached() {
        let connection = SqliteConnection::establish(":memory:").unwrap();
        let one_as_expr = 1.into_sql::<Integer>();
        let query = ::select(one_as_expr.eq_any(::select(one_as_expr)));

        assert_eq!(Ok(true), query.get_result(&connection));
        assert_eq!(1, connection.statement_cache.len());
    }

    use sql_types::Text;
    sql_function!(fn fun_case(x: Text) -> Text);

    #[test]
    fn register_custom_function() {
        let connection = SqliteConnection::establish(":memory:").unwrap();
        fun_case::register_impl(&connection, |x: String| {
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

        let mapped_string = ::select(fun_case("foobar"))
            .get_result::<String>(&connection)
            .unwrap();
        assert_eq!("fOoBaR", mapped_string);
    }

    sql_function!(fn my_add(x: Integer, y: Integer) -> Integer);

    #[test]
    fn register_multiarg_function() {
        let connection = SqliteConnection::establish(":memory:").unwrap();
        my_add::register_impl(&connection, |x: i32, y: i32| x + y).unwrap();

        let added = ::select(my_add(1, 2)).get_result::<i32>(&connection);
        assert_eq!(Ok(3), added);
    }

    sql_function!(fn add_counter(x: Integer) -> Integer);

    #[test]
    fn register_nondeterministic_function() {
        let connection = SqliteConnection::establish(":memory:").unwrap();
        let mut y = 0;
        add_counter::register_nondeterministic_impl(&connection, move |x: i32| {
            y += 1;
            x + y
        })
        .unwrap();

        let added = ::select((add_counter(1), add_counter(1), add_counter(1)))
            .get_result::<(i32, i32, i32)>(&connection);
        assert_eq!(Ok((2, 3, 4)), added);
    }
}
