extern crate libsqlite3_sys as ffi;

use std::collections::HashMap;
use std::os::raw as libc;
use std::ptr::NonNull;
use std::{slice, str};

use row::*;
use sqlite::Sqlite;

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

    pub fn read_text(&self) -> &str {
        unsafe {
            let ptr = ffi::sqlite3_value_text(self.value());
            let len = ffi::sqlite3_value_bytes(self.value());
            let bytes = slice::from_raw_parts(ptr as *const u8, len as usize);
            str::from_utf8_unchecked(bytes)
        }
    }

    pub fn read_blob(&self) -> &[u8] {
        unsafe {
            let ptr = ffi::sqlite3_value_blob(self.value());
            let len = ffi::sqlite3_value_bytes(self.value());
            slice::from_raw_parts(ptr as *const u8, len as usize)
        }
    }

    pub fn read_integer(&self) -> i32 {
        unsafe { ffi::sqlite3_value_int(self.value()) as i32 }
    }

    pub fn read_long(&self) -> i64 {
        unsafe { ffi::sqlite3_value_int64(self.value()) as i64 }
    }

    pub fn read_double(&self) -> f64 {
        unsafe { ffi::sqlite3_value_double(self.value()) as f64 }
    }

    pub fn is_null(&self) -> bool {
        let tpe = unsafe { ffi::sqlite3_value_type(self.value()) };
        tpe == ffi::SQLITE_NULL
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
