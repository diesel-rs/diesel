extern crate libsqlite3_sys as ffi;

use std::ffi::CStr;
use std::os::raw as libc;

// The sqlite3_value_*type*-functions are only safe for so-called "protected" values.

// Per https://www.sqlite.org/c3ref/value.html, values sent to application-defined functions are protected,
// making it safe to use those values with FromSqliteValue.

pub trait FromSqliteValue {
    fn from_sqlite_value(value: *mut ffi::sqlite3_value) -> Self;
}

impl FromSqliteValue for i32 {
    fn from_sqlite_value(value: *mut ffi::sqlite3_value) -> Self {
        unsafe {
            ffi::sqlite3_value_int(value)
        }
    }
}

impl FromSqliteValue for i64 {
    fn from_sqlite_value(value: *mut ffi::sqlite3_value) -> Self {
        unsafe {
            ffi::sqlite3_value_int64(value)
        }
    }
}

impl FromSqliteValue for f64 {
    fn from_sqlite_value(value: *mut ffi::sqlite3_value) -> Self {
        unsafe {
            ffi::sqlite3_value_double(value)
        }
    }
}

impl FromSqliteValue for String {
    fn from_sqlite_value(value: *mut ffi::sqlite3_value) -> Self {
        use std::{slice, str};

        unsafe {
            let len = ffi::sqlite3_value_bytes(value); // Must not be called after sqlite3_value_text
            let ptr = ffi::sqlite3_value_text(value);
            assert!(!ptr.is_null());

            // The buffer must be copied immediately (https://www.sqlite.org/c3ref/value_blob.html):
            // >Please pay particular attention to the fact that the pointer returned from ... sqlite3_value_text(), ...
            // >can be invalidated by a subsequent call to sqlite3_value_bytes(), sqlite3_value_bytes16(),
            // >sqlite3_value_text(), or sqlite3_value_text16().

            let buf = slice::from_raw_parts(ptr, len as usize);

            str::from_utf8(buf)
                .expect("SQLite guarantees UTF-8 encoding")
                .to_string()
        }
    }
}

impl FromSqliteValue for Vec<u8> {
    fn from_sqlite_value(value: *mut ffi::sqlite3_value) -> Self {
        use std::slice;

        unsafe {
            let len = ffi::sqlite3_value_bytes(value); // Must not be called after sqlite3_value_blob
            let ptr = ffi::sqlite3_value_blob(value);
            assert!(!ptr.is_null());

            // The buffer must be copied immediately (https://www.sqlite.org/c3ref/value_blob.html):
            // >Please pay particular attention to the fact that the pointer returned from ... sqlite3_value_blob(), ...
            // >can be invalidated by a subsequent call to sqlite3_value_bytes(), sqlite3_value_bytes16(),
            // >sqlite3_value_text(), or sqlite3_value_text16().

            let buf = slice::from_raw_parts(ptr as *const u8, len as usize);

            buf.into()
        }
    }
}
