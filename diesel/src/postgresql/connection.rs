use postgresql::libc;
use postgresql::db_result::PgDbResult;
use postgresql::query_builder::PgQueryBuilder;
use postgresql::pq_sys::*;

use expression::{AsExpression, Expression, NonAggregate};
use expression::expression_methods::*;
use expression::helper_types::AsExpr;
use persistable::{Insertable, InsertableColumns};
use helper_types::{FindBy, Limit};
use query_builder::{AsQuery, Query, QueryFragment};
use query_dsl::{FilterDsl, LimitDsl};
use query_source::{Table, Column, Queriable};
use result::*;
use std::cell::Cell;
use std::ffi::{CString, CStr};
use std::{str, ptr};
use types::{NativeSqlType, ToSql, IsNull};

use connection::{Cursor, Connection, PrimaryKey, PkType, FindPredicate};
use db_result::DbResult;

pub struct PgConnection {
    internal_connection: *mut PGconn,
    transaction_depth: Cell<i32>,
}

unsafe impl Send for PgConnection {}

impl PgConnection {

    fn exec_sql_params(&self, query: &str, 
                       param_data: &Vec<Option<Vec<u8>>>, 
                       param_types: &Option<Vec<u32>>) -> QueryResult<PgDbResult> {
        let query = try!(CString::new(query));
        let params_pointer = param_data.iter()
            .map(|data| data.as_ref().map(|d| d.as_ptr() as *const libc::c_char)
                 .unwrap_or(ptr::null()))
            .collect::<Vec<_>>();
        let param_types_ptr = param_types.as_ref()
            .map(|types| types.as_ptr())
            .unwrap_or(ptr::null());
        let param_lengths = param_data.iter()
            .map(|data| data.as_ref().map(|d| d.len() as libc::c_int)
                 .unwrap_or(0))
            .collect::<Vec<_>>();
        let param_formats = vec![1; param_data.len()];

        let internal_res = unsafe {
            PQexecParams(
                self.internal_connection,
                query.as_ptr(),
                params_pointer.len() as libc::c_int,
                param_types_ptr,
                params_pointer.as_ptr(),
                param_lengths.as_ptr(),
                param_formats.as_ptr(),
                1,
            )
        };

        PgDbResult::new(self, internal_res)
    }

    fn prepare_query<T: QueryFragment>(&self, source: &T)
        -> (String, Vec<Option<Vec<u8>>>, Vec<u32>)
    {
        let mut query_builder = PgQueryBuilder::new(self);
        source.to_sql(&mut query_builder).unwrap();
        (query_builder.sql, query_builder.binds, query_builder.bind_types)
    }

    fn execute_inner(&self, query: &str) -> QueryResult<PgDbResult> {
        self.exec_sql_params(query, &Vec::new(), &None)
    }

    fn placeholders_for_insert<T, U>(&self, records: U)
        -> (String, Vec<Option<Vec<u8>>>, Vec<u32>) where
        T: Table,
        U: Insertable<T>,
    {
        let mut query_builder = PgQueryBuilder::new(self);
        records.values().to_insert_sql(&mut query_builder).unwrap();
        (query_builder.sql, query_builder.binds, query_builder.bind_types)
    }

    fn begin_transaction(&self) -> QueryResult<usize> {
        let transaction_depth = self.transaction_depth.get();
        self.change_transaction_depth(1, if transaction_depth == 0 {
            self.execute("BEGIN")
        } else {
            self.execute(&format!("SAVEPOINT diesel_savepoint_{}", transaction_depth))
        })
    }

    fn rollback_transaction(&self) -> QueryResult<usize> {
        let transaction_depth = self.transaction_depth.get();
        self.change_transaction_depth(-1, if transaction_depth == 1 {
            self.execute("ROLLBACK")
        } else {
            self.execute(&format!("ROLLBACK TO SAVEPOINT diesel_savepoint_{}",
                                  transaction_depth - 1))
        })
    }

    fn commit_transaction(&self) -> QueryResult<usize> {
        let transaction_depth = self.transaction_depth.get();
        self.change_transaction_depth(-1, if transaction_depth <= 1 {
            self.execute("COMMIT")
        } else {
            self.execute(&format!("RELEASE SAVEPOINT diesel_savepoint_{}",
                                  transaction_depth - 1))
        })
    }

    fn change_transaction_depth(&self, by: i32, query: QueryResult<usize>) -> QueryResult<usize> {
        if query.is_ok() {
            self.transaction_depth.set(self.transaction_depth.get() + by);
        }
        query
    }
    
    #[doc(hidden)]
    pub fn escape_identifier(&self, identifier: &str) -> QueryResult<PgString> {
        let result_ptr = unsafe { PQescapeIdentifier(
            self.internal_connection,
            identifier.as_ptr() as *const libc::c_char,
            identifier.len() as libc::size_t,
        ) };

        if result_ptr.is_null() {
            Err(Error::DatabaseError(last_error_message(self.internal_connection)))
        } else {
            unsafe {
                Ok(PgString::new(result_ptr))
            }
        }
    }

}

fn last_error_message(conn: *const PGconn) -> String {
    unsafe {
        let error_ptr = PQerrorMessage(conn);
        let bytes = CStr::from_ptr(error_ptr).to_bytes();
        str::from_utf8_unchecked(bytes).to_string()
    }
}

impl Drop for PgConnection {
    fn drop(&mut self) {
        unsafe { PQfinish(self.internal_connection) };
    }
}

#[doc(hidden)]
pub struct PgString {
    pg_str: *mut libc::c_char,
}

impl PgString {
    unsafe fn new(ptr: *mut libc::c_char) -> Self {
        PgString {
            pg_str: ptr,
        }
    }
}

impl ::std::ops::Deref for PgString {
    type Target = str;

    fn deref(&self) -> &str {
        unsafe {
            let c_string = CStr::from_ptr(self.pg_str);
            str::from_utf8_unchecked(c_string.to_bytes())
        }
    }
}

impl Drop for PgString {
    fn drop(&mut self) {
        unsafe {
            PQfreemem(self.pg_str as *mut libc::c_void)
        }
    }
}

impl Connection for PgConnection {

    type DbResult = PgDbResult;

    fn last_error_message(&self) -> String {
        last_error_message(self.internal_connection)
    }

    fn establish(database_url: &str) -> ConnectionResult<PgConnection> {
        let connection_string = try!(CString::new(database_url));
        let connection_ptr = unsafe { PQconnectdb(connection_string.as_ptr()) };
        let connection_status = unsafe { PQstatus(connection_ptr) };
        match connection_status {
            CONNECTION_OK => {
                Ok(PgConnection {
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

    fn transaction<T, E, F>(&self, f: F) -> TransactionResult<T, E> where
        F: FnOnce() -> Result<T, E>,
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

    fn begin_test_transaction(&self) -> QueryResult<usize> {
        assert_eq!(self.transaction_depth.get(), 0);
        self.begin_transaction()
    }

    fn test_transaction<T, E, F>(&self, f: F) -> T where
        F: FnOnce() -> Result<T, E>,
    {
        let mut user_result = None;
        let _ = self.transaction::<(), _, _>(|| {
            user_result = f().ok();
            Err(())
        });
        user_result.expect("Transaction did not succeed")
    }

    fn execute(&self, query: &str) -> QueryResult<usize> {
        self.execute_inner(query).map(|res| res.rows_affected())
    }

    fn batch_execute(&self, query: &str) -> QueryResult<()> {
        let query = try!(CString::new(query));
        let inner_result = unsafe {
            PQexec(self.internal_connection, query.as_ptr())
        };
        try!(PgDbResult::new(self, inner_result));
        Ok(())
    }

    fn query_one<T, U>(&self, source: T) -> QueryResult<U> where
        T: AsQuery,
        U: Queriable<T::SqlType>,
    {
        self.query_all(source)
            .and_then(|mut e| e.nth(0).map(Ok).unwrap_or(Err(Error::NotFound)))
    }

    fn query_all<T, U>(&self, source: T) -> QueryResult<Cursor<T::SqlType, U, PgDbResult>> where
        T: AsQuery,
        U: Queriable<T::SqlType>,
    {
        let (sql, params, types) = self.prepare_query(&source.as_query());
        self.exec_sql_params(&sql, &params, &Some(types)).map(Cursor::new)
    }

    fn query_sql<T, U>(&self, query: &str) -> QueryResult<Cursor<T, U, PgDbResult>> where
        T: NativeSqlType,
        U: Queriable<T>,
    {
        let result = try!(self.execute_inner(query));
        Ok(Cursor::new(result))
    }

    fn query_sql_params<T, U, PT, P>(&self, query: &str, params: &P)
        -> QueryResult<Cursor<T, U, PgDbResult>> where
        T: NativeSqlType,
        U: Queriable<T>,
        PT: NativeSqlType,
        P: ToSql<PT>,
    {
        let mut param_data = Vec::new();
        let p = match params.to_sql(&mut param_data).unwrap() {
            IsNull::Yes => vec![None::<Vec<u8>>],
            IsNull::No => vec![Some(param_data)],
        };
        let db_result = try!(self.exec_sql_params(query, &p, &None));
        Ok(Cursor::new(db_result))
    }

    fn find<T, U, PK>(&self, source: T, id: PK) -> QueryResult<U> where
        T: Table + FilterDsl<FindPredicate<T, PK>>,
        FindBy<T, T::PrimaryKey, PK>: LimitDsl,
        U: Queriable<<Limit<FindBy<T, T::PrimaryKey, PK>> as Query>::SqlType>,
        PK: AsExpression<PkType<T>>,
        AsExpr<PK, T::PrimaryKey>: NonAggregate,
    {
        let pk = source.primary_key();
        self.query_one(source.filter(pk.eq(id)).limit(1))
    }

    fn insert<T, U, Out>(&self, _source: &T, records: U)
        -> QueryResult<Cursor<<T::AllColumns as Expression>::SqlType, Out, PgDbResult>> where
        T: Table,
        U: Insertable<T>,
        Out: Queriable<<T::AllColumns as Expression>::SqlType>,
    {
        let (param_placeholders, params, param_types) = self.placeholders_for_insert(records);
        let (returning, _, _) = self.prepare_query(&T::all_columns());
        let sql = format!(
            "INSERT INTO {} ({}) VALUES {} RETURNING {}",
            T::name(),
            U::columns().names(),
            param_placeholders,
            returning,
        );
        self.exec_sql_params(&sql, &params, &Some(param_types)).map(Cursor::new)
    }

    fn insert_returning_count<T, U>(&self, _source: &T, records: U)
        -> QueryResult<usize> where
        T: Table,
        U: Insertable<T>,
    {
        let (param_placeholders, params, param_types) = self.placeholders_for_insert(records);
        let sql = format!(
            "INSERT INTO {} ({}) VALUES {}",
            T::name(),
            U::columns().names(),
            &param_placeholders,
        );
        self.exec_sql_params(&sql, &params, &Some(param_types)).map(|r| r.rows_affected())
    }

    fn execute_returning_count<T>(&self, source: &T) -> QueryResult<usize> where
        T: QueryFragment,
    {
        let (sql, params, param_types) = self.prepare_query(source);
        self.exec_sql_params(&sql, &params, &Some(param_types))
            .map(|r| r.rows_affected())
    }

}

