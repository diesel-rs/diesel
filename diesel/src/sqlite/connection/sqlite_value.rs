extern crate libsqlite3_sys as ffi;

use std::cell::Cell;
use std::collections::HashMap;
use std::os::raw as libc;
use std::{slice, str};

use sqlite::Sqlite;
use row::*;

#[allow(missing_debug_implementations, missing_copy_implementations)]
pub struct SqliteValue {
    inner_statement: *mut ffi::sqlite3_stmt,
    col_index: Cell<libc::c_int>,
}

pub struct SqliteRow {
    value: SqliteValue,
    next_col_index: libc::c_int,
}

impl SqliteValue {
    pub fn new(inner_statement: *mut ffi::sqlite3_stmt) -> Self {
        SqliteValue {
            inner_statement: inner_statement,
            col_index: Cell::new(0),
        }
    }

    pub fn read_text(&self) -> &str {
        unsafe {
            let ptr = ffi::sqlite3_column_text(self.inner_statement, self.col_index.get());
            let len = ffi::sqlite3_column_bytes(self.inner_statement, self.col_index.get());
            let bytes = slice::from_raw_parts(ptr as *const u8, len as usize);
            str::from_utf8_unchecked(bytes)
        }
    }

    pub fn read_blob(&self) -> &[u8] {
        unsafe {
            let ptr = ffi::sqlite3_column_blob(self.inner_statement, self.col_index.get());
            let len = ffi::sqlite3_column_bytes(self.inner_statement, self.col_index.get());
            slice::from_raw_parts(ptr as *const u8, len as usize)
        }
    }

    pub fn read_integer(&self) -> i32 {
        unsafe { ffi::sqlite3_column_int(self.inner_statement, self.col_index.get()) as i32 }
    }

    pub fn read_long(&self) -> i64 {
        unsafe { ffi::sqlite3_column_int64(self.inner_statement, self.col_index.get()) as i64 }
    }

    pub fn read_double(&self) -> f64 {
        unsafe { ffi::sqlite3_column_double(self.inner_statement, self.col_index.get()) as f64 }
    }

    pub fn is_null(&self) -> bool {
        let tpe = unsafe { ffi::sqlite3_column_type(self.inner_statement, self.col_index.get()) };
        tpe == ffi::SQLITE_NULL
    }
}

impl SqliteRow {
    pub fn new(inner_statement: *mut ffi::sqlite3_stmt) -> Self {
        SqliteRow {
            value: SqliteValue::new(inner_statement),
            next_col_index: 0,
        }
    }

    pub fn into_named<'a>(self, indices: &'a HashMap<&'a str, usize>) -> SqliteNamedRow<'a> {
        SqliteNamedRow {
            value: self.value,
            column_indices: indices,
        }
    }
}

impl Row<Sqlite> for SqliteRow {
    fn take(&mut self) -> Option<&SqliteValue> {
        let is_null = self.next_is_null(1);
        self.value.col_index.set(self.next_col_index);
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

pub struct SqliteNamedRow<'a> {
    value: SqliteValue,
    column_indices: &'a HashMap<&'a str, usize>,
}

impl<'a> NamedRow<Sqlite> for SqliteNamedRow<'a> {
    fn index_of(&self, column_name: &str) -> Option<usize> {
        self.column_indices.get(column_name).cloned()
    }

    fn get_raw_value(&self, idx: usize) -> Option<&SqliteValue> {
        self.value.col_index.set(idx as libc::c_int);
        if self.value.is_null() {
            None
        } else {
            Some(&self.value)
        }
    }
}
