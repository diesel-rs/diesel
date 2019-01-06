extern crate libsqlite3_sys as ffi;

use std::ffi::{CStr, CString};
use std::io::{stderr, Write};
use std::os::raw as libc;
use std::ptr::{self, NonNull};

use super::raw::RawConnection;
use super::serialized_value::SerializedValue;
use super::sqlite_value::SqliteRow;
use crate::result::Error::DatabaseError;
use crate::result::*;
use crate::sqlite::SqliteType;

pub struct Statement {
    inner_statement: NonNull<ffi::sqlite3_stmt>,
    bind_index: libc::c_int,
}

impl Statement {
    pub fn prepare(raw_connection: &RawConnection, sql: &str) -> QueryResult<Self> {
        let mut stmt = ptr::null_mut();
        let mut unused_portion = ptr::null();
        let prepare_result = unsafe {
            ffi::sqlite3_prepare_v2(
                raw_connection.internal_connection.as_ptr(),
                CString::new(sql)?.as_ptr(),
                sql.len() as libc::c_int,
                &mut stmt,
                &mut unused_portion,
            )
        };

        ensure_sqlite_ok(prepare_result, raw_connection.internal_connection.as_ptr()).map(|_| {
            Statement {
                inner_statement: unsafe { NonNull::new_unchecked(stmt) },
                bind_index: 0,
            }
        })
    }

    fn run(&mut self) -> QueryResult<()> {
        self.step().map(|_| ())
    }

    pub fn bind(&mut self, tpe: SqliteType, value: Option<Vec<u8>>) -> QueryResult<()> {
        self.bind_index += 1;
        let value = SerializedValue {
            ty: tpe,
            data: value,
        };
        let result = value.bind_to(self.inner_statement, self.bind_index);

        ensure_sqlite_ok(result, self.raw_connection())
    }

    fn num_fields(&self) -> usize {
        unsafe { ffi::sqlite3_column_count(self.inner_statement.as_ptr()) as usize }
    }

    /// The lifetime of the returned CStr is shorter than self. This function
    /// should be tied to a lifetime that ends before the next call to `reset`
    unsafe fn field_name<'a>(&self, idx: usize) -> Option<&'a CStr> {
        let ptr = ffi::sqlite3_column_name(self.inner_statement.as_ptr(), idx as libc::c_int);
        if ptr.is_null() {
            None
        } else {
            Some(CStr::from_ptr(ptr))
        }
    }

    fn step(&mut self) -> QueryResult<Option<SqliteRow>> {
        match unsafe { ffi::sqlite3_step(self.inner_statement.as_ptr()) } {
            ffi::SQLITE_DONE => Ok(None),
            ffi::SQLITE_ROW => Ok(Some(SqliteRow::new(self.inner_statement))),
            _ => Err(last_error(self.raw_connection())),
        }
    }

    fn reset(&mut self) {
        self.bind_index = 0;
        unsafe { ffi::sqlite3_reset(self.inner_statement.as_ptr()) };
    }

    fn raw_connection(&self) -> *mut ffi::sqlite3 {
        unsafe { ffi::sqlite3_db_handle(self.inner_statement.as_ptr()) }
    }
}

fn ensure_sqlite_ok(code: libc::c_int, raw_connection: *mut ffi::sqlite3) -> QueryResult<()> {
    if code == ffi::SQLITE_OK {
        Ok(())
    } else {
        Err(last_error(raw_connection))
    }
}

fn last_error(raw_connection: *mut ffi::sqlite3) -> Error {
    let error_message = last_error_message(raw_connection);
    let error_information = Box::new(error_message);
    let error_kind = match last_error_code(raw_connection) {
        ffi::SQLITE_CONSTRAINT_UNIQUE | ffi::SQLITE_CONSTRAINT_PRIMARYKEY => {
            DatabaseErrorKind::UniqueViolation
        }
        ffi::SQLITE_CONSTRAINT_FOREIGNKEY => DatabaseErrorKind::ForeignKeyViolation,
        _ => DatabaseErrorKind::__Unknown,
    };
    DatabaseError(error_kind, error_information)
}

fn last_error_message(conn: *mut ffi::sqlite3) -> String {
    let c_str = unsafe { CStr::from_ptr(ffi::sqlite3_errmsg(conn)) };
    c_str.to_string_lossy().into_owned()
}

fn last_error_code(conn: *mut ffi::sqlite3) -> libc::c_int {
    unsafe { ffi::sqlite3_extended_errcode(conn) }
}

impl Drop for Statement {
    fn drop(&mut self) {
        use std::thread::panicking;

        let finalize_result = unsafe { ffi::sqlite3_finalize(self.inner_statement.as_ptr()) };
        if let Err(e) = ensure_sqlite_ok(finalize_result, self.raw_connection()) {
            if panicking() {
                write!(
                    stderr(),
                    "Error finalizing SQLite prepared statement: {:?}",
                    e
                )
                .expect("Error writing to `stderr`");
            } else {
                panic!("Error finalizing SQLite prepared statement: {:?}", e);
            }
        }
    }
}

pub struct StatementUse<'a> {
    statement: &'a mut Statement,
}

impl<'a> StatementUse<'a> {
    pub fn new(statement: &'a mut Statement) -> Self {
        StatementUse {
            statement: statement,
        }
    }

    pub fn run(&mut self) -> QueryResult<()> {
        self.statement.run()
    }

    pub fn step(&mut self) -> QueryResult<Option<SqliteRow>> {
        self.statement.step()
    }

    pub fn num_fields(&self) -> usize {
        self.statement.num_fields()
    }

    pub fn field_name(&self, idx: usize) -> Option<&'a CStr> {
        unsafe { self.statement.field_name(idx) }
    }
}

impl<'a> Drop for StatementUse<'a> {
    fn drop(&mut self) {
        self.statement.reset();
    }
}
