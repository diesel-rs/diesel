extern crate libsqlite3_sys as ffi;
extern crate libc;

#[doc(hidden)]
pub mod raw;
mod stmt;
mod statement_iterator;
mod sqlite_value;

pub use self::sqlite_value::SqliteValue;

use std::any::TypeId;
use std::borrow::Cow;
use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::rc::Rc;

use connection::{SimpleConnection, Connection};
use query_builder::*;
use query_builder::bind_collector::RawBytesBindCollector;
use query_source::*;
use result::*;
use self::raw::RawConnection;
use self::statement_iterator::StatementIterator;
use self::stmt::{Statement, StatementUse};
use sqlite::Sqlite;
use super::query_builder::SqliteQueryBuilder;
use types::HasSqlType;

#[allow(missing_debug_implementations)]
pub struct SqliteConnection {
    statement_cache: RefCell<HashMap<QueryCacheKey, StatementUse>>,
    raw_connection: Rc<RawConnection>,
    transaction_depth: Cell<i32>,
}

#[derive(Hash, PartialEq, Eq)]
enum QueryCacheKey {
    Sql(String),
    Type(TypeId),
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

    fn establish(database_url: &str) -> ConnectionResult<Self> {
        RawConnection::establish(database_url).map(|conn| {
            SqliteConnection {
                statement_cache: RefCell::new(HashMap::new()),
                raw_connection: Rc::new(conn),
                transaction_depth: Cell::new(0),
            }
        })
    }

    #[doc(hidden)]
    fn execute(&self, query: &str) -> QueryResult<usize> {
        try!(self.batch_execute(query));
        Ok(self.raw_connection.rows_affected_by_last_query())
    }

    #[doc(hidden)]
    fn query_all<T, U>(&self, source: T) -> QueryResult<Vec<U>> where
        T: AsQuery,
        T::Query: QueryFragment<Self::Backend> + QueryId,
        Self::Backend: HasSqlType<T::SqlType>,
        U: Queryable<T::SqlType, Self::Backend>,
    {
        let statement = try!(self.prepare_query(&source.as_query()));
        let mut statement_ref = statement.borrow_mut();
        StatementIterator::new(&mut statement_ref).collect()
    }

    #[doc(hidden)]
    fn execute_returning_count<T>(&self, source: &T) -> QueryResult<usize> where
        T: QueryFragment<Self::Backend> + QueryId,
    {
        let stmt = try!(self.prepare_query(source));
        try!(stmt.borrow().run());
        Ok(self.raw_connection.rows_affected_by_last_query())
    }

    #[doc(hidden)]
    fn silence_notices<F: FnOnce() -> T, T>(&self, f: F) -> T {
        f()
    }

    #[doc(hidden)]
    fn begin_transaction(&self) -> QueryResult<()> {
        let transaction_depth = self.transaction_depth.get();
        self.change_transaction_depth(1, if transaction_depth == 0 {
            self.execute("BEGIN")
        } else {
            self.execute(&format!("SAVEPOINT diesel_savepoint_{}", transaction_depth))
        })
    }

    #[doc(hidden)]
    fn rollback_transaction(&self) -> QueryResult<()> {
        let transaction_depth = self.transaction_depth.get();
        self.change_transaction_depth(-1, if transaction_depth == 1 {
            self.execute("ROLLBACK")
        } else {
            self.execute(&format!("ROLLBACK TO SAVEPOINT diesel_savepoint_{}",
                                  transaction_depth - 1))
        })
    }

    #[doc(hidden)]
    fn commit_transaction(&self) -> QueryResult<()> {
        let transaction_depth = self.transaction_depth.get();
        self.change_transaction_depth(-1, if transaction_depth <= 1 {
            self.execute("COMMIT")
        } else {
            self.execute(&format!("RELEASE SAVEPOINT diesel_savepoint_{}",
                                  transaction_depth - 1))
        })
    }

    #[doc(hidden)]
    fn get_transaction_depth(&self) -> i32 {
        self.transaction_depth.get()
    }

    #[doc(hidden)]
    fn setup_helper_functions(&self) {
        // this will be implemented at least when timestamps are supported in SQLite
    }
}

impl SqliteConnection {
    fn prepare_query<T: QueryFragment<Sqlite> + QueryId>(&self, source: &T) -> QueryResult<StatementUse> {
        let result = try!(self.cached_prepared_statement(source));

        let mut bind_collector = RawBytesBindCollector::<Sqlite>::new();
        try!(source.collect_binds(&mut bind_collector));
        {
            let mut stmt = result.borrow_mut();
            for (tpe, value) in bind_collector.binds.into_iter() {
                try!(stmt.bind(tpe, value));
            }
        }

        Ok(result)
    }

    fn change_transaction_depth(&self, by: i32, query: QueryResult<usize>) -> QueryResult<()> {
        if query.is_ok() {
            self.transaction_depth.set(self.transaction_depth.get() + by);
        }
        query.map(|_| ())
    }

    fn cached_prepared_statement<T: QueryFragment<Sqlite> + QueryId>(&self, source: &T)
        -> QueryResult<StatementUse>
    {
        use std::collections::hash_map::Entry::{Occupied, Vacant};

        let cache_key = try!(cache_key(source));
        let mut cache = self.statement_cache.borrow_mut();

        match cache.entry(cache_key) {
            Occupied(entry) => Ok(entry.get().clone()),
            Vacant(entry) => {
                let statement = {
                    let sql = try!(sql_from_cache_key(&entry.key(), source));

                    Statement::prepare(&self.raw_connection, &sql)
                        .map(StatementUse::new)
                };

                if !source.is_safe_to_cache_prepared() {
                    return statement;
                }

                Ok(entry.insert(try!(statement)).clone())
            }
        }
    }
}

fn cache_key<T: QueryFragment<Sqlite> + QueryId>(source: &T)
    -> QueryResult<QueryCacheKey>
{
    match T::query_id() {
        Some(id) => Ok(QueryCacheKey::Type(id)),
        None => to_sql(source).map(QueryCacheKey::Sql),
    }
}

fn sql_from_cache_key<'a, T: QueryFragment<Sqlite>>(key: &'a QueryCacheKey, source: &T)
    -> QueryResult<Cow<'a, str>>
{
    match key {
        &QueryCacheKey::Sql(ref sql) => Ok(Cow::Borrowed(sql)),
        _ => to_sql(source).map(Cow::Owned),
    }
}

fn to_sql<T: QueryFragment<Sqlite>>(source: &T) -> QueryResult<String> {
    let mut query_builder = SqliteQueryBuilder::new();
    try!(source.to_sql(&mut query_builder));
    Ok(query_builder.sql)
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
        assert_eq!(1, connection.statement_cache.borrow().len());
    }

    #[test]
    fn sql_literal_nodes_are_not_cached() {
        let connection = SqliteConnection::establish(":memory:").unwrap();
        let query = ::select(sql::<Integer>("1"));

        assert_eq!(Ok(1), query.get_result(&connection));
        assert_eq!(0, connection.statement_cache.borrow().len());
    }

    #[test]
    fn queries_containing_sql_literal_nodes_are_not_cached() {
        let connection = SqliteConnection::establish(":memory:").unwrap();
        let one_as_expr = AsExpression::<Integer>::as_expression(1);
        let query = ::select(one_as_expr.eq(sql::<Integer>("1")));

        assert_eq!(Ok(true), query.get_result(&connection));
        assert_eq!(0, connection.statement_cache.borrow().len());
    }

    #[test]
    fn queries_containing_in_with_vec_are_not_cached() {
        let connection = SqliteConnection::establish(":memory:").unwrap();
        let one_as_expr = AsExpression::<Integer>::as_expression(1);
        let query = ::select(one_as_expr.eq_any(vec![1, 2, 3]));

        assert_eq!(Ok(true), query.get_result(&connection));
        assert_eq!(0, connection.statement_cache.borrow().len());
    }

    #[test]
    fn queries_containing_in_with_subselect_are_cached() {
        let connection = SqliteConnection::establish(":memory:").unwrap();
        let one_as_expr = AsExpression::<Integer>::as_expression(1);
        let query = ::select(one_as_expr.eq_any(::select(one_as_expr)));

        assert_eq!(Ok(true), query.get_result(&connection));
        assert_eq!(1, connection.statement_cache.borrow().len());
    }
}
