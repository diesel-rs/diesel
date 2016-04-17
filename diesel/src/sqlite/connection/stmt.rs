extern crate libsqlite3_sys as ffi;
extern crate libc;
extern crate byteorder;

use self::byteorder::{ReadBytesExt, BigEndian};
use std::ffi::CString;
use std::io::{stderr, Write};
use std::ptr;

use sqlite::SqliteType;
use result::*;
use result::Error::{DatabaseError, QueryBuilderError};
use super::raw::RawConnection;
use super::sqlite_value::SqliteRow;

pub struct Statement {
    inner_statement: *mut ffi::sqlite3_stmt,
    bind_index: libc::c_int,
}

impl Statement {
    pub fn prepare(raw_connection: &RawConnection, sql: &str) -> QueryResult<Self> {
        let mut stmt = ptr::null_mut();
        let mut unused_portion = ptr::null();
        let prepare_result = unsafe {
            ffi::sqlite3_prepare_v2(
                raw_connection.internal_connection,
                try!(CString::new(sql)).as_ptr(),
                sql.len() as libc::c_int,
                &mut stmt,
                &mut unused_portion,
            )
        };

        ensure_sqlite_ok(prepare_result)
            .map(|_| Statement { inner_statement: stmt, bind_index: 0 })
    }

    pub fn run(&self) -> QueryResult<()> {
        match unsafe { ffi::sqlite3_step(self.inner_statement) } {
            ffi::SQLITE_DONE | ffi::SQLITE_ROW => Ok(()),
            error => Err(DatabaseError(super::error_message(error).into()))
        }
    }

    pub fn bind(&mut self, tpe: SqliteType, value: Option<Vec<u8>>) -> QueryResult<()> {
        self.bind_index += 1;
        let result = unsafe { match (tpe, value) {
            (_, None) =>
                ffi::sqlite3_bind_null(self.inner_statement, self.bind_index),
            (SqliteType::Binary, Some(bytes)) =>
                ffi::sqlite3_bind_blob(
                    self.inner_statement,
                    self.bind_index,
                    bytes.as_ptr() as *const libc::c_void,
                    bytes.len() as libc::c_int,
                    ffi::SQLITE_TRANSIENT(),
                ),
            (SqliteType::Text, Some(bytes)) =>
                ffi::sqlite3_bind_text(
                    self.inner_statement,
                    self.bind_index,
                    bytes.as_ptr() as *const libc::c_char,
                    bytes.len() as libc::c_int,
                    ffi::SQLITE_TRANSIENT(),
                ),
            (SqliteType::Float, Some(bytes)) => {
                let value = try!((&bytes[..]).read_f32::<BigEndian>()
                    .map_err(|e| QueryBuilderError(Box::new(e))));
                ffi::sqlite3_bind_double(
                    self.inner_statement,
                    self.bind_index,
                    value as libc::c_double,
                )
            }
            (SqliteType::Double, Some(bytes)) => {
                let value = try!((&bytes[..]).read_f64::<BigEndian>()
                    .map_err(|e| QueryBuilderError(Box::new(e))));
                ffi::sqlite3_bind_double(
                    self.inner_statement,
                    self.bind_index,
                    value as libc::c_double,
                )
            }
            (SqliteType::SmallInt, Some(bytes)) => {
                let value = try!((&bytes[..]).read_i16::<BigEndian>()
                    .map_err(|e| QueryBuilderError(Box::new(e))));
                ffi::sqlite3_bind_int(
                    self.inner_statement,
                    self.bind_index,
                    value as libc::c_int,
                )
            }
            (SqliteType::Integer, Some(bytes)) => {
                let value = try!((&bytes[..]).read_i32::<BigEndian>()
                    .map_err(|e| QueryBuilderError(Box::new(e))));
                ffi::sqlite3_bind_int(
                    self.inner_statement,
                    self.bind_index,
                    value as libc::c_int,
                )
            }
            (SqliteType::Long, Some(bytes)) => {
                let value = try!((&bytes[..]).read_i64::<BigEndian>()
                    .map_err(|e| QueryBuilderError(Box::new(e))));
                ffi::sqlite3_bind_int64(
                    self.inner_statement,
                    self.bind_index,
                    value,
                )
            }
        }};

        ensure_sqlite_ok(result)
    }

    pub fn step(&mut self) -> Option<SqliteRow> {
        match unsafe { ffi::sqlite3_step(self.inner_statement) } {
            ffi::SQLITE_DONE => None,
            ffi::SQLITE_ROW => Some(SqliteRow::new(self.inner_statement)),
            error => panic!("{}", super::error_message(error)),
        }
    }

    fn reset(&mut self) -> QueryResult<()> {
        self.bind_index = 0;
        ensure_sqlite_ok(unsafe { ffi::sqlite3_reset(self.inner_statement) })
    }
}

fn ensure_sqlite_ok(code: libc::c_int) -> QueryResult<()> {
    if code != ffi::SQLITE_OK {
        Err(DatabaseError(super::error_message(code).into()))
    } else {
        Ok(())
    }
}

impl Drop for Statement {
    fn drop(&mut self) {
        use std::thread::panicking;

        let finalize_result = unsafe { ffi::sqlite3_finalize(self.inner_statement) };
        if let Err(e) = ensure_sqlite_ok(finalize_result) {
            if panicking() {
                write!(stderr(), "Error finalizing SQLite prepared statement: {:?}", e).unwrap();
            } else {
                panic!("Error finalizing SQLite prepared statement: {:?}", e);
            }
        }
    }
}

use std::cell::RefCell;
use std::ops::Deref;
use std::rc::Rc;

#[derive(Clone)]
pub struct StatementUse {
    statement: Rc<RefCell<Statement>>,
}

impl StatementUse {
    pub fn new(statement: Statement) -> Self {
        StatementUse {
            statement: Rc::new(RefCell::new(statement)),
        }
    }
}

impl Deref for StatementUse {
    type Target = RefCell<Statement>;

    fn deref(&self) -> &Self::Target {
        &*self.statement
    }
}

impl Drop for StatementUse {
    fn drop(&mut self) {
        use std::thread::panicking;

        let reset_result = self.statement.borrow_mut().reset();
        if let Err(e) = reset_result {
            if panicking() {
                write!(stderr(), "Error resetting SQLite prepared statement: {:?}", e).unwrap();
            } else {
                panic!("Error resetting SQLite prepared statement: {:?}", e);
            }
        }
    }
}
