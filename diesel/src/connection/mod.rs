extern crate pq_sys;
extern crate libc;

mod cursor;

pub use self::cursor::Cursor;

use db_result::DbResult;
use expression::{AsExpression, Expression, NonAggregate};
use expression::expression_methods::*;
use expression::predicates::Eq;
use helper_types::{FindBy, Limit};
use expression::helper_types::AsExpr;
use query_builder::{AsQuery, Query, QueryFragment};
use query_builder::pg::PgQueryBuilder;
use query_dsl::{FilterDsl, LimitDsl};
use query_source::{Table, Queryable};
use result::*;
use self::pq_sys::*;
use std::cell::Cell;
use std::ffi::{CString, CStr};
use std::{str, ptr};
use types::{NativeSqlType, ToSql};

pub struct Connection {
    internal_connection: *mut PGconn,
    transaction_depth: Cell<i32>,
}

unsafe impl Send for Connection {}

#[doc(hidden)]
pub type PrimaryKey<T> = <T as Table>::PrimaryKey;
#[doc(hidden)]
pub type PkType<T> = <PrimaryKey<T> as Expression>::SqlType;
#[doc(hidden)]
pub type FindPredicate<T, PK> = Eq<PrimaryKey<T>, <PK as AsExpression<PkType<T>>>::Expression>;

impl Connection {
    /// Establishes a new connection to the database at the given URL. The URL
    /// should be a PostgreSQL connection string, as documented at
    /// http://www.postgresql.org/docs/9.4/static/libpq-connect.html#LIBPQ-CONNSTRING
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

    /// Executes the given function inside of a database transaction. When
    /// a transaction is already occurring,
    /// [savepoints](http://www.postgresql.org/docs/9.1/static/sql-savepoint.html)
    /// will be used to emulate a nested transaction.
    ///
    /// If the function returns an `Ok`, that value will be returned.  If the
    /// function returns an `Err`,
    /// [`TransactionError::UserReturnedError`](result/enum.TransactionError.html#variant.UserReturnedError)
    /// will be returned wrapping that value.
    pub fn transaction<T, E, F>(&self, f: F) -> TransactionResult<T, E> where
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

    /// Creates a transaction that will never be committed. This is useful for
    /// tests. Panics if called while inside of a transaction.
    pub fn begin_test_transaction(&self) -> QueryResult<usize> {
        assert_eq!(self.transaction_depth.get(), 0);
        self.begin_transaction()
    }

    /// Executes the given function inside a transaction, but does not commit
    /// it. Panics if the given function returns an `Err`.
    pub fn test_transaction<T, E, F>(&self, f: F) -> T where
        F: FnOnce() -> Result<T, E>,
    {
        let mut user_result = None;
        let _ = self.transaction::<(), _, _>(|| {
            user_result = f().ok();
            Err(())
        });
        user_result.expect("Transaction did not succeed")
    }

    #[doc(hidden)]
    pub fn execute(&self, query: &str) -> QueryResult<usize> {
        self.execute_inner(query).map(|res| res.rows_affected())
    }

    #[doc(hidden)]
    pub fn batch_execute(&self, query: &str) -> QueryResult<()> {
        let query = try!(CString::new(query));
        let inner_result = unsafe {
            PQexec(self.internal_connection, query.as_ptr())
        };
        try!(DbResult::new(self, inner_result));
        Ok(())
    }

    #[doc(hidden)]
    pub fn query_one<T, U>(&self, source: T) -> QueryResult<U> where
        T: AsQuery,
        U: Queryable<T::SqlType>,
    {
        self.query_all(source)
            .and_then(|mut e| e.nth(0).map(Ok).unwrap_or(Err(Error::NotFound)))
    }

    #[doc(hidden)]
    pub fn query_all<T, U>(&self, source: T) -> QueryResult<Cursor<T::SqlType, U>> where
        T: AsQuery,
        U: Queryable<T::SqlType>,
    {
        let (sql, params, types) = self.prepare_query(&source.as_query());
        self.exec_sql_params(&sql, &params, &Some(types)).map(Cursor::new)
    }

    fn exec_sql_params(&self, query: &str, param_data: &Vec<Option<Vec<u8>>>, param_types: &Option<Vec<u32>>) -> QueryResult<DbResult> {
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

        DbResult::new(self, internal_res)
    }

    /// Attempts to find a single record from the given table by primary key.
    ///
    /// # Example
    ///
    /// ```rust
    /// # #[macro_use] extern crate diesel;
    /// # include!("src/doctest_setup.rs");
    /// #
    /// # table! {
    /// #     users {
    /// #         id -> Serial,
    /// #         name -> VarChar,
    /// #     }
    /// # }
    /// #
    /// # fn main() {
    /// #     use self::users::dsl::*;
    /// #     use diesel::result::Error::NotFound;
    /// #     let connection = establish_connection();
    /// let sean = (1, "Sean".to_string());
    /// let tess = (2, "Tess".to_string());
    /// assert_eq!(Ok(sean), connection.find(users, 1));
    /// assert_eq!(Ok(tess), connection.find(users, 2));
    /// assert_eq!(Err::<(i32, String), _>(NotFound), connection.find(users, 3));
    /// # }
    /// ```
    pub fn find<T, U, PK>(&self, source: T, id: PK) -> QueryResult<U> where
        T: Table + FilterDsl<FindPredicate<T, PK>>,
        FindBy<T, T::PrimaryKey, PK>: LimitDsl,
        U: Queryable<<Limit<FindBy<T, T::PrimaryKey, PK>> as Query>::SqlType>,
        PK: AsExpression<PkType<T>>,
        AsExpr<PK, T::PrimaryKey>: NonAggregate,
    {
        let pk = source.primary_key();
        self.query_one(source.filter(pk.eq(id)).limit(1))
    }

    #[doc(hidden)]
    pub fn execute_returning_count<T>(&self, source: &T) -> QueryResult<usize> where
        T: QueryFragment,
    {
        let (sql, params, param_types) = self.prepare_query(source);
        self.exec_sql_params(&sql, &params, &Some(param_types))
            .map(|r| r.rows_affected())
    }

    fn prepare_query<T: QueryFragment>(&self, source: &T)
        -> (String, Vec<Option<Vec<u8>>>, Vec<u32>)
    {
        let mut query_builder = PgQueryBuilder::new(self);
        source.to_sql(&mut query_builder).unwrap();
        (query_builder.sql, query_builder.binds, query_builder.bind_types)
    }

    fn execute_inner(&self, query: &str) -> QueryResult<DbResult> {
        self.exec_sql_params(query, &Vec::new(), &None)
    }

    #[doc(hidden)]
    pub fn last_error_message(&self) -> String {
        last_error_message(self.internal_connection)
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

    #[doc(hidden)]
    pub fn silence_notices<F: FnOnce() -> T, T>(&self, f: F) -> T {
        unsafe { PQsetNoticeProcessor(self.internal_connection, Some(noop_notice_processor), ptr::null_mut()) };
        let result = f();
        unsafe { PQsetNoticeProcessor(self.internal_connection, Some(default_notice_processor), ptr::null_mut()) };
        result
    }
}

extern "C" fn noop_notice_processor(_: *mut libc::c_void, _message: *const libc::c_char) {
}

extern "C" fn default_notice_processor(_: *mut libc::c_void, message: *const libc::c_char) {
    use std::io::Write;
    let c_str = unsafe { CStr::from_ptr(message) };
    ::std::io::stderr().write(c_str.to_bytes()).unwrap();
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
