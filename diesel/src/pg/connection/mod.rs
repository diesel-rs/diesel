mod cursor;
pub mod raw;
mod row;
#[doc(hidden)]
pub mod result;
mod stmt;

use std::ffi::{CString, CStr};
use std::os::raw as libc;

use connection::*;
use pg::Pg;
use query_builder::*;
use query_builder::bind_collector::RawBytesBindCollector;
use query_source::Queryable;
use result::*;
use self::cursor::Cursor;
use self::raw::RawConnection;
use self::result::PgResult;
use self::stmt::Statement;
use types::HasSqlType;

/// The connection string expected by `PgConnection::establish`
/// should be a PostgreSQL connection string, as documented at
/// http://www.postgresql.org/docs/9.4/static/libpq-connect.html#LIBPQ-CONNSTRING
#[allow(missing_debug_implementations)]
pub struct PgConnection {
    raw_connection: RawConnection,
    transaction_manager: AnsiTransactionManager,
    statement_cache: StatementCache<Pg, Statement>,
}

unsafe impl Send for PgConnection {}

impl SimpleConnection for PgConnection {
    fn batch_execute(&self, query: &str) -> QueryResult<()> {
        let query = try!(CString::new(query));
        let inner_result = unsafe {
            self.raw_connection.exec(query.as_ptr())
        };
        try!(PgResult::new(inner_result?));
        Ok(())
    }
}

impl Connection for PgConnection {
    type Backend = Pg;
    type TransactionManager = AnsiTransactionManager;

    fn establish(database_url: &str) -> ConnectionResult<PgConnection> {
        RawConnection::establish(database_url).map(|raw_conn| {
            PgConnection {
                raw_connection: raw_conn,
                transaction_manager: AnsiTransactionManager::new(),
                statement_cache: StatementCache::new(),
            }
        })
    }

    #[doc(hidden)]
    fn execute(&self, query: &str) -> QueryResult<usize> {
        self.execute_inner(query).map(|res| res.rows_affected())
    }

    #[doc(hidden)]
    fn query_by_index<T, U>(&self, source: T) -> QueryResult<Vec<U>> where
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
    fn transaction_manager(&self) -> &Self::TransactionManager {
        &self.transaction_manager
    }

    #[doc(hidden)]
    fn setup_helper_functions(&self) {
        self.batch_execute(
            include_str!("setup/timestamp_helpers.sql")
        ).expect("Error creating timestamp helper functions for Pg");
    }
}

impl PgConnection {
    #[cfg_attr(feature = "clippy", allow(type_complexity))]
    fn prepare_query<T: QueryFragment<Pg> + QueryId>(&self, source: &T)
        -> QueryResult<(MaybeCached<Statement>, Vec<Option<Vec<u8>>>)>
    {
        let mut bind_collector = RawBytesBindCollector::<Pg>::new();
        try!(source.collect_binds(&mut bind_collector));
        let binds = bind_collector.binds;
        let metadata = bind_collector.metadata;

        let cache_len = self.statement_cache.len();
        let query = self.statement_cache.cached_statement(source, &metadata, |sql| {
            let query_name = if source.is_safe_to_cache_prepared()? {
                Some(format!("__diesel_stmt_{}", cache_len))
            } else {
                None
            };
            Statement::prepare(
                &self.raw_connection,
                sql,
                query_name.as_ref().map(|s| &**s),
                &metadata,
            )
        });

        Ok((query?, binds))
    }

    fn execute_inner(&self, query: &str) -> QueryResult<PgResult> {
        let query = try!(Statement::prepare(&self.raw_connection, query, None, &[]));
        query.execute(&self.raw_connection, &Vec::new())
    }
}

extern "C" fn noop_notice_processor(_: *mut libc::c_void, _message: *const libc::c_char) {
}

extern "C" fn default_notice_processor(_: *mut libc::c_void, message: *const libc::c_char) {
    use std::io::Write;
    let c_str = unsafe { CStr::from_ptr(message) };
    ::std::io::stderr()
        .write_all(c_str.to_bytes())
        .expect("Error writing to `stderr`");
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
    fn queries_with_identical_sql_but_different_types_are_cached_separately() {
        let connection = connection();

        let query = ::select(AsExpression::<Integer>::as_expression(1));
        let query2 = ::select(AsExpression::<VarChar>::as_expression("hi"));

        assert_eq!(Ok(1), query.get_result(&connection));
        assert_eq!(Ok("hi".to_string()), query2.get_result(&connection));
        assert_eq!(2, connection.statement_cache.len());
    }

    #[test]
    fn queries_with_identical_types_and_sql_but_different_bind_types_are_cached_separately() {
        let connection = connection();

        let query = ::select(AsExpression::<Integer>::as_expression(1)).into_boxed::<Pg>();
        let query2 = ::select(AsExpression::<VarChar>::as_expression("hi")).into_boxed::<Pg>();

        assert_eq!(0, connection.statement_cache.len());
        assert_eq!(Ok(1), query.get_result(&connection));
        assert_eq!(Ok("hi".to_string()), query2.get_result(&connection));
        assert_eq!(2, connection.statement_cache.len());
    }

    #[test]
    fn queries_with_identical_types_and_binds_but_different_sql_are_cached_separately() {
        let connection = connection();

        sql_function!(lower, lower_t, (x: VarChar) -> VarChar);
        let hi = AsExpression::<VarChar>::as_expression("HI");
        let query = ::select(hi).into_boxed::<Pg>();
        let query2 = ::select(lower(hi)).into_boxed::<Pg>();

        assert_eq!(0, connection.statement_cache.len());
        assert_eq!(Ok("HI".to_string()), query.get_result(&connection));
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
        let database_url = env::var("PG_DATABASE_URL")
            .or_else(|_| env::var("DATABASE_URL"))
            .expect("DATABASE_URL must be set in order to run tests");
        PgConnection::establish(&database_url).unwrap()
    }
}
