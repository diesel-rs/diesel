extern crate libsqlite3_sys as ffi;

use std::collections::HashMap;
use std::os::raw as libc;
use std::ptr::NonNull;
use std::{slice, str};

use crate::row::*;
use crate::sqlite::{Sqlite, SqliteType};

/// Raw sqlite value as received from the database
///
/// Use existing `FromSql` implementations to convert this into
/// rust values:
#[allow(missing_debug_implementations, missing_copy_implementations)]
pub struct SqliteValue {
    value: ffi::sqlite3_value,
}

pub struct SqliteRow {
    stmt: NonNull<ffi::sqlite3_stmt>,
    next_col_index: libc::c_int,
}

impl SqliteValue {
    #[allow(clippy::new_ret_no_self)]
    pub(crate) unsafe fn new<'a>(inner: *mut ffi::sqlite3_value) -> Option<&'a Self> {
        (inner as *const _ as *const Self).as_ref().and_then(|v| {
            if v.is_null() {
                None
            } else {
                Some(v)
            }
        })
    }

    pub(crate) fn read_text(&self) -> &str {
        unsafe {
            let ptr = ffi::sqlite3_value_text(self.value());
            let len = ffi::sqlite3_value_bytes(self.value());
            let bytes = slice::from_raw_parts(ptr as *const u8, len as usize);
            // The string is guaranteed to be utf8 according to
            // https://www.sqlite.org/c3ref/value_blob.html
            str::from_utf8_unchecked(bytes)
        }
    }

    pub(crate) fn read_blob(&self) -> &[u8] {
        unsafe {
            let ptr = ffi::sqlite3_value_blob(self.value());
            let len = ffi::sqlite3_value_bytes(self.value());
            slice::from_raw_parts(ptr as *const u8, len as usize)
        }
    }

    pub(crate) fn read_integer(&self) -> i32 {
        unsafe { ffi::sqlite3_value_int(self.value()) as i32 }
    }

    pub(crate) fn read_long(&self) -> i64 {
        unsafe { ffi::sqlite3_value_int64(self.value()) as i64 }
    }

    pub(crate) fn read_double(&self) -> f64 {
        unsafe { ffi::sqlite3_value_double(self.value()) as f64 }
    }

    pub(crate) fn is_null(&self) -> bool {
        self.value_type().is_none()
    }

    /// Get the type of the value as returned by sqlite
    pub fn value_type(&self) -> Option<SqliteType> {
        let tpe = unsafe { ffi::sqlite3_value_type(self.value()) };
        match tpe {
            ffi::SQLITE_TEXT => Some(SqliteType::Text),
            ffi::SQLITE_INTEGER => Some(SqliteType::Long),
            ffi::SQLITE_FLOAT => Some(SqliteType::Double),
            ffi::SQLITE_BLOB => Some(SqliteType::Binary),
            ffi::SQLITE_NULL => None,
            _ => unreachable!("Sqlite does saying this is not reachable"),
        }
    }

    fn value(&self) -> *mut ffi::sqlite3_value {
        &self.value as *const _ as _
    }
}

impl SqliteRow {
    pub(crate) fn new(inner_statement: NonNull<ffi::sqlite3_stmt>) -> Self {
        SqliteRow {
            stmt: inner_statement,
            next_col_index: 0,
        }
    }

    pub fn into_named<'a>(self, indices: &'a HashMap<&'a str, usize>) -> SqliteNamedRow<'a> {
        SqliteNamedRow {
            stmt: self.stmt,
            column_indices: indices,
        }
    }
}

impl Row<Sqlite> for SqliteRow {
    fn take(&mut self) -> Option<&SqliteValue> {
        let col_index = self.next_col_index;
        self.next_col_index += 1;

        unsafe {
            let ptr = ffi::sqlite3_column_value(self.stmt.as_ptr(), col_index);
            SqliteValue::new(ptr)
        }
    }

    fn next_is_null(&self, count: usize) -> bool {
        (0..count).all(|i| {
            let idx = self.next_col_index + i as libc::c_int;
            let tpe = unsafe { ffi::sqlite3_column_type(self.stmt.as_ptr(), idx) };
            tpe == ffi::SQLITE_NULL
        })
    }

    fn column_name(&self) -> Option<&str> {
        unsafe {
            let ptr = ffi::sqlite3_column_name(self.stmt.as_ptr(), self.next_col_index);
            Some(std::ffi::CStr::from_ptr(ptr).to_str().expect(
                "The Sqlite documentation states that this is UTF8. \
                 If you see this error message something has gone \
                 horribliy wrong. Please open an issue at the \
                 diesel repository.",
            ))
        }
    }

    fn column_count(&self) -> usize {
        unsafe { ffi::sqlite3_column_count(self.stmt.as_ptr()) as usize }
    }
}

pub struct SqliteNamedRow<'a> {
    stmt: NonNull<ffi::sqlite3_stmt>,
    column_indices: &'a HashMap<&'a str, usize>,
}

impl<'a> NamedRow<Sqlite> for SqliteNamedRow<'a> {
    fn index_of(&self, column_name: &str) -> Option<usize> {
        self.column_indices.get(column_name).cloned()
    }

    fn get_raw_value(&self, idx: usize) -> Option<&SqliteValue> {
        unsafe {
            let ptr = ffi::sqlite3_column_value(self.stmt.as_ptr(), idx as libc::c_int);
            SqliteValue::new(ptr)
        }
    }
}
