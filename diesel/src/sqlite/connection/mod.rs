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
use std::cell::{RefCell, RefMut};
use std::collections::HashMap;
use std::rc::Rc;

use connection::{SimpleConnection, Connection, AnsiTransactionManager};
use query_builder::*;
use query_builder::bind_collector::RawBytesBindCollector;
use query_source::*;
use result::*;
use result::Error::QueryBuilderError;
use self::raw::RawConnection;
use self::statement_iterator::StatementIterator;
use self::stmt::{Statement, StatementUse};
use sqlite::Sqlite;
use super::query_builder::SqliteQueryBuilder;
use types::HasSqlType;

#[allow(missing_debug_implementations)]
pub struct SqliteConnection {
    statement_cache: RefCell<HashMap<QueryCacheKey, Statement>>,
    raw_connection: Rc<RawConnection>,
    transaction_manager: AnsiTransactionManager,
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
    type TransactionManager = AnsiTransactionManager;

    fn establish(database_url: &str) -> ConnectionResult<Self> {
        RawConnection::establish(database_url).map(|conn| {
            SqliteConnection {
                statement_cache: RefCell::new(HashMap::new()),
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
    fn query_all<T, U>(&self, source: T) -> QueryResult<Vec<U>> where
        T: AsQuery,
        T::Query: QueryFragment<Self::Backend> + QueryId,
        Self::Backend: HasSqlType<T::SqlType>,
        U: Queryable<T::SqlType, Self::Backend>,
    {
        let mut statement = try!(self.prepare_query(&source.as_query()));
        let statement_use = StatementUse::new(&mut statement);
        let x = StatementIterator::new(statement_use).collect();
        x
    }

    #[doc(hidden)]
    fn execute_returning_count<T>(&self, source: &T) -> QueryResult<usize> where
        T: QueryFragment<Self::Backend> + QueryId,
    {
        let stmt = try!(self.prepare_query(source));
        try!(stmt.run());
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
    fn prepare_query<T: QueryFragment<Sqlite> + QueryId>(&self, source: &T)
        -> QueryResult<RefMut<Statement>>
    {
        let mut stmt = try!(self.cached_prepared_statement(source));

        let mut bind_collector = RawBytesBindCollector::<Sqlite>::new();
        try!(source.collect_binds(&mut bind_collector));
        {
            for (tpe, value) in bind_collector.binds.into_iter() {
                try!(stmt.bind(tpe, value));
            }
        }

        Ok(stmt)
    }

    fn cached_prepared_statement<T: QueryFragment<Sqlite> + QueryId>(&self, source: &T)
        -> QueryResult<RefMut<Statement>>
    {
        use std::collections::hash_map::Entry::{Occupied, Vacant};

        refmut_map_result(self.statement_cache.borrow_mut(), |cache| {
            match cache.entry(cache_key(source)?) {
                Occupied(entry) => Ok(entry.into_mut()),
                Vacant(entry) => {
                    let statement = {
                        let sql = try!(sql_from_cache_key(&entry.key(), source));

                        Statement::prepare(&self.raw_connection, &sql)
                    };

                    // if !source.is_safe_to_cache_prepared() {
                    //     return statement;
                    // }

                    Ok(entry.insert(try!(statement)))
                }
            }
        })
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
    try!(source.to_sql(&mut query_builder).map_err(QueryBuilderError));
    Ok(query_builder.sql)
}

fn error_message(err_code: libc::c_int) -> &'static str {
    ffi::code_to_str(err_code)
}

fn refmut_map_result<T, U, E, F>(refmut: RefMut<T>, f: F) -> Result<RefMut<U>, E> where
    F: FnOnce(&mut T) -> Result<&mut U, E>,
{
    use std::mem;

    let mut error = None;
    let refmut = RefMut::map(refmut, |mutref| match f(mutref) {
        Ok(x) => x,
        Err(e) => {
            error = Some(e);
            unsafe { mem::uninitialized() }
        }
    });
    match error {
        Some(e) => Err(e),
        None => Ok(refmut),
    }
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
