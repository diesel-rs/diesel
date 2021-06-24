extern crate libsqlite3_sys as ffi;

use super::raw::RawConnection;
use super::serialized_value::SerializedValue;
use super::sqlite_value::OwnedSqliteValue;
use crate::result::Error::DatabaseError;
use crate::result::*;
use crate::sqlite::SqliteType;
use crate::util::OnceCell;
use core::slice;
use std::ffi::{CStr, CString};
use std::io::{stderr, Write};
use std::os::raw as libc;
use std::ptr::{self, NonNull};

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

    pub fn bind(&mut self, tpe: SqliteType, value: Option<Vec<u8>>) -> QueryResult<()> {
        self.bind_index += 1;
        let value = SerializedValue {
            ty: tpe,
            data: value,
        };
        let result = value.bind_to(self.inner_statement, self.bind_index);

        ensure_sqlite_ok(result, self.raw_connection())
    }

    fn reset(&mut self) {
        self.bind_index = 0;
        unsafe { ffi::sqlite3_reset(self.inner_statement.as_ptr()) };
    }

    fn raw_connection(&self) -> *mut ffi::sqlite3 {
        unsafe { ffi::sqlite3_db_handle(self.inner_statement.as_ptr()) }
    }
}

pub(super) fn ensure_sqlite_ok(
    code: libc::c_int,
    raw_connection: *mut ffi::sqlite3,
) -> QueryResult<()> {
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
        ffi::SQLITE_CONSTRAINT_NOTNULL => DatabaseErrorKind::NotNullViolation,
        ffi::SQLITE_CONSTRAINT_CHECK => DatabaseErrorKind::CheckViolation,
        _ => DatabaseErrorKind::Unknown,
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

        let raw_connection = self.raw_connection();
        let finalize_result = unsafe { ffi::sqlite3_finalize(self.inner_statement.as_ptr()) };
        if let Err(e) = ensure_sqlite_ok(finalize_result, raw_connection) {
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

#[allow(missing_debug_implementations)]
pub struct StatementUse<'a> {
    statement: &'a mut Statement,
    column_names: OnceCell<Vec<*const str>>,
}

impl<'a> StatementUse<'a> {
    pub(in crate::sqlite::connection) fn new(statement: &'a mut Statement) -> Self {
        StatementUse {
            statement,
            column_names: OnceCell::new(),
        }
    }

    pub(in crate::sqlite::connection) fn run(self) -> QueryResult<()> {
        self.step().map(|_| ())
    }

    pub(in crate::sqlite::connection) fn step<'c>(self) -> QueryResult<Option<Self>> {
        let res = unsafe {
            match ffi::sqlite3_step(self.statement.inner_statement.as_ptr()) {
                ffi::SQLITE_DONE => Ok(None),
                ffi::SQLITE_ROW => Ok(Some(())),
                _ => Err(last_error(self.statement.raw_connection())),
            }
        }?;
        Ok(res.map(move |()| self))
    }

    // The returned string pointer is valid until either the prepared statement is
    // destroyed by sqlite3_finalize() or until the statement is automatically
    // reprepared by the first call to sqlite3_step() for a particular run or
    // until the next call to sqlite3_column_name() or sqlite3_column_name16()
    // on the same column.
    //
    // https://sqlite.org/c3ref/column_name.html
    unsafe fn column_name(&mut self, idx: i32) -> *const str {
        let name = {
            let column_name =
                ffi::sqlite3_column_name(self.statement.inner_statement.as_ptr(), idx);
            assert!(
                !column_name.is_null(),
                "The Sqlite documentation states that it only returns a \
                 null pointer here if we are in a OOM condition."
            );
            CStr::from_ptr(column_name)
        };
        name.to_str().expect(
            "The Sqlite documentation states that this is UTF8. \
             If you see this error message something has gone \
             horribliy wrong. Please open an issue at the \
             diesel repository.",
        ) as *const str
    }

    pub(super) fn column_count(&self) -> i32 {
        unsafe { ffi::sqlite3_column_count(self.statement.inner_statement.as_ptr()) }
    }

    pub(super) fn index_for_column_name(&mut self, field_name: &str) -> Option<usize> {
        (0..self.column_count())
            .find(|idx| self.field_name(*idx) == Some(field_name))
            .map(|v| v as usize)
    }

    pub(super) fn field_name<'c>(&'c mut self, idx: i32) -> Option<&'c str> {
        if let Some(column_names) = self.column_names.get() {
            return column_names
                .get(idx as usize)
                .and_then(|c| unsafe { c.as_ref() });
        }
        let values = (0..self.column_count())
            .map(|idx| unsafe { self.column_name(idx) })
            .collect::<Vec<_>>();
        let ret = values.get(idx as usize).copied();
        let _ = self.column_names.set(values);
        ret.and_then(|p| unsafe { p.as_ref() })
    }

    pub(super) fn column_type(&self, idx: i32) -> Option<SqliteType> {
        let tpe = unsafe { ffi::sqlite3_column_type(self.statement.inner_statement.as_ptr(), idx) };
        SqliteType::from_raw_sqlite(tpe)
    }

    pub(super) fn read_column_as_str(&self, idx: i32) -> &str {
        unsafe {
            let ptr = ffi::sqlite3_column_text(self.statement.inner_statement.as_ptr(), idx);
            let len = ffi::sqlite3_column_bytes(self.statement.inner_statement.as_ptr(), idx);
            let bytes = slice::from_raw_parts(ptr as *const u8, len as usize);
            // The string is guaranteed to be utf8 according to
            // https://www.sqlite.org/c3ref/value_blob.html
            std::str::from_utf8_unchecked(bytes)
        }
    }

    pub(super) fn read_column_as_blob(&self, idx: i32) -> &[u8] {
        unsafe {
            let ptr = ffi::sqlite3_column_blob(self.statement.inner_statement.as_ptr(), idx);
            let len = ffi::sqlite3_column_bytes(self.statement.inner_statement.as_ptr(), idx);
            slice::from_raw_parts(ptr as *const u8, len as usize)
        }
    }

    pub(super) fn read_column_as_integer(&self, idx: i32) -> i32 {
        unsafe { ffi::sqlite3_column_int(self.statement.inner_statement.as_ptr(), idx) }
    }

    pub(super) fn read_column_as_long(&self, idx: i32) -> i64 {
        unsafe { ffi::sqlite3_column_int64(self.statement.inner_statement.as_ptr(), idx) }
    }

    pub(super) fn read_column_as_double(&self, idx: i32) -> f64 {
        unsafe { ffi::sqlite3_column_double(self.statement.inner_statement.as_ptr(), idx) }
    }

    pub(super) fn copy_value(&self, idx: i32) -> Option<OwnedSqliteValue> {
        let ptr =
            unsafe { ffi::sqlite3_column_value(self.statement.inner_statement.as_ptr(), idx) };
        OwnedSqliteValue::copy_from_ptr(ptr)
    }
}

impl<'a> Drop for StatementUse<'a> {
    fn drop(&mut self) {
        self.statement.reset();
    }
}
