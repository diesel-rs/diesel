extern crate libc;

mod cursor;
pub mod raw;
mod row;
#[doc(hidden)]
pub mod result;
mod stmt;

use std::cell::Cell;
use std::ffi::{CString, CStr};
use std::rc::Rc;

use connection::{SimpleConnection, Connection};
use pg::{Pg, PgQueryBuilder};
use query_builder::{AsQuery, QueryFragment, QueryId};
use query_builder::bind_collector::RawBytesBindCollector;
use query_source::Queryable;
use result::*;
use self::cursor::Cursor;
use self::raw::RawConnection;
use self::result::PgResult;
use self::stmt::{Query, StatementCache};
use types::HasSqlType;

/// The connection string expected by `PgConnection::establish`
/// should be a PostgreSQL connection string, as documented at
/// http://www.postgresql.org/docs/9.4/static/libpq-connect.html#LIBPQ-CONNSTRING
#[allow(missing_debug_implementations)]
pub struct PgConnection {
    raw_connection: Rc<RawConnection>,
    transaction_depth: Cell<i32>,
    statement_cache: StatementCache,
}

unsafe impl Send for PgConnection {}

impl SimpleConnection for PgConnection {
    fn batch_execute(&self, query: &str) -> QueryResult<()> {
        let query = try!(CString::new(query));
        let inner_result = unsafe {
            self.raw_connection.exec(query.as_ptr())
        };
        try!(PgResult::new(inner_result));
        Ok(())
    }
}

impl Connection for PgConnection {
    type Backend = Pg;

    fn establish(database_url: &str) -> ConnectionResult<PgConnection> {
        RawConnection::establish(database_url).map(|raw_conn| {
            PgConnection {
                raw_connection: Rc::new(raw_conn),
                transaction_depth: Cell::new(0),
                statement_cache: StatementCache::new(),
            }
        })
    }

    #[doc(hidden)]
    fn execute(&self, query: &str) -> QueryResult<usize> {
        self.execute_inner(query).map(|res| res.rows_affected())
    }

    #[doc(hidden)]
    fn query_all<T, U>(&self, source: T) -> QueryResult<Vec<U>> where
        T: AsQuery,
        T::Query: QueryFragment<Pg> + QueryId,
        Pg: HasSqlType<T::SqlType>,
        U: Queryable<T::SqlType, Pg>,
    {
        let (query, params) = try!(self.prepare_query(&source.as_query()));
        query.execute(&self.raw_connection, &params)
            .and_then(|r| Cursor::new(r).collect())
    }

    #[doc(hidden)]
    fn execute_returning_count<T>(&self, source: &T) -> QueryResult<usize> where
        T: QueryFragment<Pg> + QueryId,
    {
        let (query, params) = try!(self.prepare_query(source));
        query.execute(&self.raw_connection, &params)
            .map(|r| r.rows_affected())
    }

    #[doc(hidden)]
    fn silence_notices<F: FnOnce() -> T, T>(&self, f: F) -> T {
        self.raw_connection.set_notice_processor(noop_notice_processor);
        let result = f();
        self.raw_connection.set_notice_processor(default_notice_processor);
        result
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
        self.batch_execute(
            include_str!("setup/timestamp_helpers.sql")
        ).expect("Error creating timestamp helper functions for Pg");
    }
}

impl PgConnection {
    fn prepare_query<T: QueryFragment<Pg> + QueryId>(&self, source: &T)
        -> QueryResult<(Rc<Query>, Vec<Option<Vec<u8>>>)>
    {
        let mut bind_collector = RawBytesBindCollector::<Pg>::new();
        source.collect_binds(&mut bind_collector).unwrap();
        let (binds, bind_types) = bind_collector.binds.into_iter()
            .map(|(meta, bind)| (bind, meta.oid)).unzip();

        let query = if source.is_safe_to_cache_prepared() {
            try!(self.statement_cache.cached_query(
                &self.raw_connection,
                source,
                bind_types,
            ))
        } else {
            let mut query_builder = PgQueryBuilder::new(&self.raw_connection);
            try!(source.to_sql(&mut query_builder));
            Rc::new(try!(Query::sql(&query_builder.sql, Some(bind_types))))
        };

        Ok((query, binds))
    }

    fn execute_inner(&self, query: &str) -> QueryResult<PgResult> {
        let query = try!(Query::sql(query, None));
        query.execute(&self.raw_connection, &Vec::new())
    }

    fn change_transaction_depth(&self, by: i32, query: QueryResult<usize>) -> QueryResult<()> {
        if query.is_ok() {
            self.transaction_depth.set(self.transaction_depth.get() + by);
        }
        query.map(|_| ())
    }
}

extern "C" fn noop_notice_processor(_: *mut libc::c_void, _message: *const libc::c_char) {
}

extern "C" fn default_notice_processor(_: *mut libc::c_void, message: *const libc::c_char) {
    use std::io::Write;
    let c_str = unsafe { CStr::from_ptr(message) };
    ::std::io::stderr().write(c_str.to_bytes()).unwrap();
}

#[cfg(test)]
mod tests {
    extern crate dotenv;

    use self::dotenv::dotenv;
    use std::env;

    use expression::AsExpression;
    use expression::dsl::sql;
    use prelude::*;
    use super::*;
    use types::{Integer, VarChar};

    #[test]
    fn prepared_statements_are_cached() {
        let connection = connection();

        let query = ::select(AsExpression::<Integer>::as_expression(1));

        assert_eq!(Ok(1), query.get_result(&connection));
        assert_eq!(Ok(1), query.get_result(&connection));
        assert_eq!(1, connection.statement_cache.len());
    }

    #[test]
    fn different_queries_have_unique_names() {
        let connection = connection();

        let one = AsExpression::<Integer>::as_expression(1);
        let query = ::select(one);
        let query2 = ::select(one.eq(one));

        assert_eq!(Ok(1), query.get_result(&connection));
        assert_eq!(Ok(true), query2.get_result(&connection));

        let statement_names = connection.statement_cache.statement_names();
        assert_eq!(2, statement_names.len());
    }

    #[test]
    fn queries_with_identical_sql_but_different_types_are_cached_separately() {
        let connection = connection();

        let query = ::select(AsExpression::<Integer>::as_expression(1));
        let query2 = ::select(AsExpression::<VarChar>::as_expression("hi"));

        assert_eq!(Ok(1), query.get_result(&connection));
        assert_eq!(Ok("hi".to_string()), query2.get_result(&connection));
        assert_eq!(2, connection.statement_cache.len());
    }

    #[test]
    fn queries_with_sql_literal_nodes_are_not_cached() {
        let connection = connection();
        let query = ::select(sql::<Integer>("1"));

        assert_eq!(Ok(1), query.get_result(&connection));
        assert_eq!(0, connection.statement_cache.len());
    }

    fn connection() -> PgConnection {
        dotenv().ok();
        let database_url = env::var("DATABASE_URL").unwrap();
        PgConnection::establish(&database_url).unwrap()
    }
}
