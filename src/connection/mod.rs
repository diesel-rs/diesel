extern crate pq_sys;

mod cursor;

pub use self::cursor::Cursor;

use {Result, ConnectionResult, ConnectionError, QuerySource, Queriable};
use db_result::DbResult;
use types::NativeSqlType;
use std::ffi::{CString, CStr};
use std::{str, ptr};

use self::pq_sys::*;

pub struct Connection {
    internal_connection: *mut PGconn,
}

impl Connection {
    pub fn establish(database_url: &str) -> ConnectionResult<Connection> {
        let connection_string = try!(CString::new(database_url));
        let connection_ptr = unsafe { PQconnectdb(connection_string.as_ptr()) };
        let connection_status = unsafe { PQstatus(connection_ptr) };
        match connection_status {
            CONNECTION_OK => {
                Ok(Connection {
                    internal_connection: connection_ptr,
                })
            },
            _ => {
                let message = last_error_message(connection_ptr);
                Err(ConnectionError::BadConnection(message))
            }
        }
    }

    pub fn execute(&self, query: &str) -> Result<usize> {
        self.execute_inner(query).map(|res| res.rows_affected())
    }

    pub fn query_one<T, U>(&self, source: &T) -> Result<Option<U>> where
        T: QuerySource,
        U: Queriable<T::SqlType>,
    {
        self.query_all(source).map(|mut e| e.nth(0))
    }

    pub fn query_all<T, U>(&self, source: &T) -> Result<Cursor<T::SqlType, U>> where
        T: QuerySource,
        U: Queriable<T::SqlType>,
    {
        let sql = self.prepare_query(source);
        self.query_sql(&sql)
    }

    pub fn query_sql<T, U>(&self, query: &str) -> Result<Cursor<T, U>> where
        T: NativeSqlType,
        U: Queriable<T>,
    {
        let result = try!(self.execute_inner(query));
        Ok(Cursor::new(result))
    }

    fn prepare_query<T: QuerySource>(&self, source: &T) -> String {
        format!("SELECT {} FROM {}", source.select_clause(), source.from_clause())
    }

    fn execute_inner(&self, query: &str) -> Result<DbResult> {
        let query = try!(CString::new(query));
        let internal_res = unsafe {
            PQexecParams(
                self.internal_connection,
                query.as_ptr(),
                0,
                ptr::null(),
                ptr::null(),
                ptr::null(),
                ptr::null(),
                1,
           )
        };
        DbResult::new(self, internal_res)
    }

    pub fn last_error_message(&self) -> String {
        last_error_message(self.internal_connection)
    }
}

fn last_error_message(conn: *const PGconn) -> String {
    unsafe {
        let error_ptr = PQerrorMessage(conn);
        let bytes = CStr::from_ptr(error_ptr).to_bytes();
        str::from_utf8_unchecked(bytes).to_string()
    }
}

impl Drop for Connection {
    fn drop(&mut self) {
        unsafe { PQfinish(self.internal_connection) };
    }
}
