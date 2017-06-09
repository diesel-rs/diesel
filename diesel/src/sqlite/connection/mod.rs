extern crate libsqlite3_sys as ffi;

#[doc(hidden)]
pub mod raw;
mod stmt;
mod statement_iterator;
mod sqlite_value;

pub use self::sqlite_value::SqliteValue;

use std::os::raw as libc;
use std::rc::Rc;

use connection::*;
use query_builder::*;
use query_builder::bind_collector::RawBytesBindCollector;
use query_source::*;
use result::*;
use self::raw::RawConnection;
use self::statement_iterator::StatementIterator;
use self::stmt::{Statement, StatementUse};
use sqlite::Sqlite;
use types::HasSqlType;

#[allow(missing_debug_implementations)]
pub struct SqliteConnection {
    statement_cache: StatementCache<Sqlite, Statement>,
    raw_connection: Rc<RawConnection>,
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
        RawConnection::establish(database_url).map(|conn| {
            SqliteConnection {
                statement_cache: StatementCache::new(),
                raw_connection: Rc::new(conn),
                transaction_manager: AnsiTransactionManager::new(),
            }
        })
    }

    #[doc(hidden)]
    fn execute(&self, query: &str) -> QueryResult<usize> {
        try!(self.batch_execute(query));
        Ok(self.raw_connection.rows_affected_by_last_query())
    }

    #[doc(hidden)]
    fn query_by_index<T, U>(&self, source: T) -> QueryResult<Vec<U>> where
        T: AsQuery,
        T::Query: QueryFragment<Self::Backend> + QueryId,
        Self::Backend: HasSqlType<T::SqlType>,
        U: Queryable<T::SqlType, Self::Backend>,
    {
        let mut statement = try!(self.prepare_query(&source.as_query()));
        let statement_use = StatementUse::new(&mut statement);
        let iter = StatementIterator::new(statement_use);
        iter.collect()
    }

    #[doc(hidden)]
    fn execute_returning_count<T>(&self, source: &T) -> QueryResult<usize> where
        T: QueryFragment<Self::Backend> + QueryId,
    {
        let mut statement = try!(self.prepare_query(source));
        let statement_use = StatementUse::new(&mut statement);
        try!(statement_use.run());
        Ok(self.raw_connection.rows_affected_by_last_query())
    }

    #[doc(hidden)]
    fn silence_notices<F: FnOnce() -> T, T>(&self, f: F) -> T {
        f()
    }

    #[doc(hidden)]
    fn transaction_manager(&self) -> &Self::TransactionManager {
        &self.transaction_manager
    }

    #[doc(hidden)]
    fn setup_helper_functions(&self) {
        // this will be implemented at least when timestamps are supported in SQLite
    }
}

impl SqliteConnection {
    fn prepare_query<T: QueryFragment<Sqlite> + QueryId>(&self, source: &T) -> QueryResult<MaybeCached<Statement>> {
        let mut statement = try!(self.cached_prepared_statement(source));

        let mut bind_collector = RawBytesBindCollector::<Sqlite>::new();
        try!(source.collect_binds(&mut bind_collector));
        let metadata = bind_collector.metadata;
        let binds = bind_collector.binds;
        for (tpe, value) in metadata.into_iter().zip(binds) {
            try!(statement.bind(tpe, value));
        }

        Ok(statement)
    }

    fn cached_prepared_statement<T: QueryFragment<Sqlite> + QueryId>(&self, source: &T)
        -> QueryResult<MaybeCached<Statement>>
    {
        self.statement_cache.cached_statement(source, &[], |sql| {
            Statement::prepare(&self.raw_connection, sql)
        })
    }
}

fn error_message(err_code: libc::c_int) -> &'static str {
    ffi::code_to_str(err_code)
}

#[cfg(test)]
mod tests {
    use expression::AsExpression;
    use expression::dsl::sql;
    use prelude::*;
    use super::*;
    use types::Integer;

    #[test]
    fn prepared_statements_are_cached_when_run() {
        let connection = SqliteConnection::establish(":memory:").unwrap();
        let query = ::select(AsExpression::<Integer>::as_expression(1));

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
        let one_as_expr = AsExpression::<Integer>::as_expression(1);
        let query = ::select(one_as_expr.eq(sql::<Integer>("1")));

        assert_eq!(Ok(true), query.get_result(&connection));
        assert_eq!(0, connection.statement_cache.len());
    }

    #[test]
    fn queries_containing_in_with_vec_are_not_cached() {
        let connection = SqliteConnection::establish(":memory:").unwrap();
        let one_as_expr = AsExpression::<Integer>::as_expression(1);
        let query = ::select(one_as_expr.eq_any(vec![1, 2, 3]));

        assert_eq!(Ok(true), query.get_result(&connection));
        assert_eq!(0, connection.statement_cache.len());
    }

    #[test]
    fn queries_containing_in_with_subselect_are_cached() {
        let connection = SqliteConnection::establish(":memory:").unwrap();
        let one_as_expr = AsExpression::<Integer>::as_expression(1);
        let query = ::select(one_as_expr.eq_any(::select(one_as_expr)));

        assert_eq!(Ok(true), query.get_result(&connection));
        assert_eq!(1, connection.statement_cache.len());
    }
}
