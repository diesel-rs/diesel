use super::ffi;

/// The sqlite3_value_*type*-functions are only safe for so-called "protected" values.
///
/// Per https://www.sqlite.org/c3ref/value.html, values sent to application-defined functions are protected,
/// making it safe to use those values with FromSqliteValue.
pub trait FromSqliteValue {
    /// * `value` must be a valid pointer
    /// * `value` must be "protected" as defined in the sqlite documentation
    fn from_sqlite_value(value: *mut ffi::sqlite3_value) -> Self;
}

impl FromSqliteValue for i32 {
    fn from_sqlite_value(value: *mut ffi::sqlite3_value) -> Self {
        unsafe { ffi::sqlite3_value_int(value) }
    }
}

impl FromSqliteValue for i64 {
    fn from_sqlite_value(value: *mut ffi::sqlite3_value) -> Self {
        unsafe { ffi::sqlite3_value_int64(value) }
    }
}

impl FromSqliteValue for f64 {
    fn from_sqlite_value(value: *mut ffi::sqlite3_value) -> Self {
        unsafe { ffi::sqlite3_value_double(value) }
    }
}

impl FromSqliteValue for String {
    fn from_sqlite_value(value: *mut ffi::sqlite3_value) -> Self {
        use std::{slice, str};

        unsafe {
            // Call sqlite3_value_text(), then _bytes(), as per https://www.sqlite.org/c3ref/column_blob.html
            let ptr = ffi::sqlite3_value_text(value);
            let len = ffi::sqlite3_value_bytes(value);
            assert!(!ptr.is_null());

            // The buffer must be copied immediately (https://www.sqlite.org/c3ref/value_blob.html):
            // >Please pay particular attention to the fact that the pointer returned from ... sqlite3_value_text(), ...
            // >can be invalidated by a subsequent call to sqlite3_value_bytes(), sqlite3_value_bytes16(),
            // >sqlite3_value_text(), or sqlite3_value_text16().

            // This copy makes this abstraction non-zero cost

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
            // Call sqlite3_value_blob(), then _bytes(), as per https://www.sqlite.org/c3ref/column_blob.html
            let ptr = ffi::sqlite3_value_blob(value);
            let len = ffi::sqlite3_value_bytes(value);
            assert!(!ptr.is_null());

            // The buffer must be copied immediately (https://www.sqlite.org/c3ref/value_blob.html):
            // >Please pay particular attention to the fact that the pointer returned from ... sqlite3_value_blob(), ...
            // >can be invalidated by a subsequent call to sqlite3_value_bytes(), sqlite3_value_bytes16(),
            // >sqlite3_value_text(), or sqlite3_value_text16().

            // This copy makes this abstraction non-zero cost

            let buf = slice::from_raw_parts(ptr as *const u8, len as usize);

            buf.into()
        }
    }
}
