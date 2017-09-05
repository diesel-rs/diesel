extern crate libsqlite3_sys as ffi;

use std::os::raw as libc;
use std::{slice, str};

use sqlite::Sqlite;
use row::Row;

#[allow(missing_debug_implementations, missing_copy_implementations)]
pub struct SqliteValue {
    inner_statement: *mut ffi::sqlite3_stmt,
    col_index: libc::c_int,
}

pub struct SqliteRow {
    value: SqliteValue,
    next_col_index: libc::c_int,
}

impl SqliteValue {
    pub fn new(inner_statement: *mut ffi::sqlite3_stmt) -> Self {
        SqliteValue {
            inner_statement: inner_statement,
            col_index: 0,
        }
    }

    pub fn read_text(&self) -> &str {
        unsafe {
            let ptr = ffi::sqlite3_column_text(self.inner_statement, self.col_index);
            let len = ffi::sqlite3_column_bytes(self.inner_statement, self.col_index);
            let bytes = slice::from_raw_parts(ptr as *const u8, len as usize);
            str::from_utf8_unchecked(bytes)
        }
    }

    pub fn read_blob(&self) -> &[u8] {
        unsafe {
            let ptr = ffi::sqlite3_column_blob(self.inner_statement, self.col_index);
            let len = ffi::sqlite3_column_bytes(self.inner_statement, self.col_index);
            slice::from_raw_parts(ptr as *const u8, len as usize)
        }
    }

    pub fn read_integer(&self) -> i32 {
        unsafe { ffi::sqlite3_column_int(self.inner_statement, self.col_index) as i32 }
    }

    pub fn read_long(&self) -> i64 {
        unsafe { ffi::sqlite3_column_int64(self.inner_statement, self.col_index) as i64 }
    }

    pub fn read_double(&self) -> f64 {
        unsafe { ffi::sqlite3_column_double(self.inner_statement, self.col_index) as f64 }
    }
}

impl SqliteRow {
    pub fn new(inner_statement: *mut ffi::sqlite3_stmt) -> Self {
        SqliteRow {
            value: SqliteValue::new(inner_statement),
            next_col_index: 0,
        }
    }
}

impl Row<Sqlite> for SqliteRow {
    fn take(&mut self) -> Option<&SqliteValue> {
        let is_null = self.next_is_null(1);
        self.value.col_index = self.next_col_index;
        self.next_col_index += 1;
        if is_null {
            None
        } else {
            Some(&self.value)
        }
    }

    fn next_is_null(&self, count: usize) -> bool {
        (0..count).all(|i| {
            let idx = self.next_col_index + i as libc::c_int;
            let tpe = unsafe { ffi::sqlite3_column_type(self.value.inner_statement, idx) };
            tpe == ffi::SQLITE_NULL
        })
    }
}
