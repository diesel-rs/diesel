extern crate pq_sys;
extern crate libc;

mod cursor;

pub use self::cursor::Cursor;

use db_result::DbResult;
use persistable::{Insertable, InsertableColumns, AsBindParam};
use query_source::{Table, Column};
use result::*;
use self::pq_sys::*;
use std::cell::Cell;
use std::ffi::{CString, CStr};
use std::{str, ptr, result};
use types::{NativeSqlType, ToSql, ValuesToSql};
use {QuerySource, Queriable};

pub struct Connection {
    internal_connection: *mut PGconn,
    transaction_depth: Cell<i32>,
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
                    transaction_depth: Cell::new(0),
                })
            },
            _ => {
                let message = last_error_message(connection_ptr);
                Err(ConnectionError::BadConnection(message))
            }
        }
    }

    pub fn transaction<T, E, F>(&self, f: F) -> TransactionResult<T, E> where
        F: FnOnce() -> result::Result<T, E>,
    {
        try!(self.begin_transaction());
        match f() {
            Ok(value) => {
                try!(self.commit_transaction());
                Ok(value)
            },
            Err(e) => {
                try!(self.rollback_transaction());
                Err(TransactionError::UserReturnedError(e))
            },
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
        let (sql, params) = self.prepare_query(source);
        self.exec_sql_params(&sql, &params).map(Cursor::new)
    }

    pub fn query_sql<T, U>(&self, query: &str) -> Result<Cursor<T, U>> where
        T: NativeSqlType,
        U: Queriable<T>,
    {
        let result = try!(self.execute_inner(query));
        Ok(Cursor::new(result))
    }

    pub fn query_sql_params<T, U, PT, P>(&self, query: &str, params: &P)
        -> Result<Cursor<T, U>> where
        T: NativeSqlType,
        U: Queriable<T>,
        PT: NativeSqlType,
        P: ValuesToSql<PT>,
    {
        let param_data = params.values_to_sql().unwrap();
        let db_result = try!(self.exec_sql_params(query, &param_data));
        Ok(Cursor::new(db_result))
    }

    fn exec_sql_params(&self, query: &str, param_data: &Vec<Option<Vec<u8>>>) -> Result<DbResult> {
        let query = try!(CString::new(query));
        let params_pointer = param_data.iter()
            .map(|data| data.as_ref().map(|d| d.as_ptr() as *const libc::c_char)
                 .unwrap_or(ptr::null()))
            .collect::<Vec<_>>();
        let param_lengths = param_data.iter()
            .map(|data| data.as_ref().map(|d| d.len() as libc::c_int)
                 .unwrap_or(0))
            .collect::<Vec<_>>();
        let param_formats = param_data.iter()
            .map(|_| 1 as libc::c_int)
            .collect::<Vec<_>>();

        let internal_res = unsafe {
            PQexecParams(
                self.internal_connection,
                query.as_ptr(),
                params_pointer.len() as libc::c_int,
                ptr::null(),
                params_pointer.as_ptr(),
                param_lengths.as_ptr(),
                param_formats.as_ptr(),
                1,
            )
        };

        DbResult::new(self, internal_res)
    }

    pub fn find<T, U, PK>(&self, source: &T, id: &PK) -> Result<Option<U>> where
        T: Table,
        U: Queriable<T::SqlType>,
        PK: ToSql<<T::PrimaryKey as Column>::SqlType>,
    {
        let (sql, binds) = self.prepare_query(source);
        assert!(binds.is_empty());
        let sql = sql + &format!(" WHERE {} = $1 LIMIT 1", source.primary_key().qualified_name());
        self.query_sql_params(&sql, id).map(|mut e| e.nth(0))
    }

    pub fn insert<T, U, Out>(&self, source: &T, records: Vec<U>)
        -> Result<Cursor<T::SqlType, Out>> where
        T: Table,
        U: Insertable<T>,
        Out: Queriable<T::SqlType>,
    {
        let (param_placeholders, params) = self.placeholders_for_insert(records);
        let sql = format!(
            "INSERT INTO {} ({}) VALUES {} RETURNING {}",
            source.name(),
            U::columns().names(),
            param_placeholders,
            source.select_clause(),
        );
        self.exec_sql_params(&sql, &params).map(Cursor::new)
    }

    pub fn insert_without_return<T, U>(&self, source: &T, records: Vec<U>)
        -> Result<usize> where
        T: Table,
        U: Insertable<T>,
    {
        let (param_placeholders, params) = self.placeholders_for_insert(records);
        let sql = format!(
            "INSERT INTO {} ({}) VALUES {}",
            source.name(),
            U::columns().names(),
            param_placeholders,
        );
        self.exec_sql_params(&sql, &params).map(|r| r.rows_affected())
    }

    fn prepare_query<T: QuerySource>(&self, source: &T) -> (String, Vec<Option<Vec<u8>>>) {
        let mut query = format!("SELECT {} FROM {}", source.select_clause(), source.from_clause());
        let mut params = Vec::new();
        if let Some((filter, mut binds)) = source.where_clause() {
            query.push_str(" WHERE ");
            query.push_str(&filter);
            params.append(&mut binds);
        }
        (query, params)
    }

    fn execute_inner(&self, query: &str) -> Result<DbResult> {
        self.exec_sql_params(query, &Vec::new())
    }

    pub fn last_error_message(&self) -> String {
        last_error_message(self.internal_connection)
    }

    fn placeholders_for_insert<T, U>(&self, records: Vec<U>)
        -> (String, Vec<Option<Vec<u8>>>) where
        T: Table,
        U: Insertable<T>,
    {
        let mut param_index = 1;
        let values: Vec<_> = records.into_iter()
            .map(|r| r.values())
            .collect();
        let param_placeholders = values.iter()
            .map(|record| { format!("({})", record.as_bind_param_for_insert(&mut param_index)) })
            .collect::<Vec<_>>()
            .join(",");
        let params = values.into_iter()
            .flat_map(|r| r.values_to_sql().unwrap()
                      .into_iter().filter(|i| i.is_some()))
            .collect();
        (param_placeholders, params)
    }

    fn begin_transaction(&self) -> Result<usize> {
        let transaction_depth = self.transaction_depth.get();
        self.change_transaction_depth(1, if transaction_depth == 0 {
            self.execute("BEGIN")
        } else {
            self.execute(&format!("SAVEPOINT yaqb_savepoint_{}", transaction_depth))
        })
    }

    fn rollback_transaction(&self) -> Result<usize> {
        let transaction_depth = self.transaction_depth.get();
        self.change_transaction_depth(-1, if transaction_depth == 1 {
            self.execute("ROLLBACK")
        } else {
            self.execute(&format!("ROLLBACK TO SAVEPOINT yaqb_savepoint_{}",
                                  transaction_depth - 1))
        })
    }

    fn commit_transaction(&self) -> Result<usize> {
        let transaction_depth = self.transaction_depth.get();
        self.change_transaction_depth(-1, if transaction_depth <= 1 {
            self.execute("COMMIT")
        } else {
            self.execute(&format!("RELEASE SAVEPOINT yaqb_savepoint_{}",
                                  transaction_depth - 1))
        })
    }

    fn change_transaction_depth(&self, by: i32, query: Result<usize>) -> Result<usize> {
        if query.is_ok() {
            self.transaction_depth.set(self.transaction_depth.get() + by);
        }
        query
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
