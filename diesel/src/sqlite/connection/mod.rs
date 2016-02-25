extern crate libsqlite3_sys as ffi;
extern crate libc;

#[doc(hidden)]
pub mod raw;
mod stmt;
mod statement_iterator;
mod sqlite_value;

pub use self::sqlite_value::SqliteValue;

use std::cell::Cell;
use std::ffi::CStr;

use connection::{SimpleConnection, Connection};
use query_builder::*;
use query_source::*;
use result::*;
use result::Error::QueryBuilderError;
use self::raw::RawConnection;
use self::statement_iterator::StatementIterator;
use self::stmt::Statement;
use sqlite::Sqlite;
use super::query_builder::SqliteQueryBuilder;
use types::HasSqlType;

pub struct SqliteConnection {
    raw_connection: RawConnection,
    transaction_depth: Cell<i32>,
}

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
                raw_connection: conn,
                transaction_depth: Cell::new(0),
            }
        })
    }

    fn execute(&self, query: &str) -> QueryResult<usize> {
        try!(self.batch_execute(query));
        Ok(self.raw_connection.rows_affected_by_last_query())
    }

    fn query_all<T, U>(&self, source: T) -> QueryResult<Vec<U>> where
        T: AsQuery,
        T::Query: QueryFragment<Self::Backend>,
        Self::Backend: HasSqlType<T::SqlType>,
        U: Queryable<T::SqlType, Self::Backend>,
    {
        self.prepare_query(&source.as_query())
            .map(StatementIterator::new)
            .and_then(Iterator::collect)
    }

    fn execute_returning_count<T>(&self, source: &T) -> QueryResult<usize> where
        T: QueryFragment<Self::Backend>,
    {
        let stmt = try!(self.prepare_query(source));
        try!(stmt.run());
        Ok(self.raw_connection.rows_affected_by_last_query())
    }

    fn silence_notices<F: FnOnce() -> T, T>(&self, f: F) -> T {
        f()
    }

    fn begin_transaction(&self) -> QueryResult<()> {
        let transaction_depth = self.transaction_depth.get();
        self.change_transaction_depth(1, if transaction_depth == 0 {
            self.execute("BEGIN")
        } else {
            self.execute(&format!("SAVEPOINT diesel_savepoint_{}", transaction_depth))
        })
    }

    fn rollback_transaction(&self) -> QueryResult<()> {
        let transaction_depth = self.transaction_depth.get();
        self.change_transaction_depth(-1, if transaction_depth == 1 {
            self.execute("ROLLBACK")
        } else {
            self.execute(&format!("ROLLBACK TO SAVEPOINT diesel_savepoint_{}",
                                  transaction_depth - 1))
        })
    }

    fn commit_transaction(&self) -> QueryResult<()> {
        let transaction_depth = self.transaction_depth.get();
        self.change_transaction_depth(-1, if transaction_depth <= 1 {
            self.execute("COMMIT")
        } else {
            self.execute(&format!("RELEASE SAVEPOINT diesel_savepoint_{}",
                                  transaction_depth - 1))
        })
    }

    fn get_transaction_depth(&self) -> i32 {
        self.transaction_depth.get()
    }

    fn setup_helper_functions(&self) {
        // this will be implemented at least when timestamps are supported in SQLite
    }
}

impl SqliteConnection {
    fn prepare_query<T: QueryFragment<Sqlite>>(&self, source: &T) -> QueryResult<Statement> {
        let mut query_builder = SqliteQueryBuilder::new();
        try!(source.to_sql(&mut query_builder).map_err(QueryBuilderError));
        let mut result = try!(Statement::prepare(&self.raw_connection, &query_builder.sql));

        for (tpe, value) in query_builder.bind_params.into_iter() {
            try!(result.bind(tpe, value));
        }

        Ok(result)
    }

    fn change_transaction_depth(&self, by: i32, query: QueryResult<usize>) -> QueryResult<()> {
        if query.is_ok() {
            self.transaction_depth.set(self.transaction_depth.get() + by);
        }
        query.map(|_| ())
    }

    #[doc(hidden)]
    pub fn execute_pragma<ST, U>(&self, source: &str) -> QueryResult<Vec<U>>
        where U: Queryable<ST, Sqlite>,
              <SqliteConnection as Connection>::Backend: HasSqlType<ST>
    {
        Statement::prepare(&self.raw_connection, source)
            .map(StatementIterator::new)
            .and_then(Iterator::collect)
    }
}

fn error_message(err_code: libc::c_int) -> &'static str {
    unsafe {
        let message_ptr = ffi::sqlite3_errstr(err_code);
        let result = CStr::from_ptr(message_ptr);
        result.to_str().unwrap()
    }
}
