#![allow(unsafe_code)] // ffi calls
#[cfg(not(all(target_family = "wasm", target_os = "unknown")))]
extern crate libsqlite3_sys as ffi;

#[cfg(all(target_family = "wasm", target_os = "unknown"))]
use sqlite_wasm_rs as ffi;

use std::cell::Ref;
use std::ptr::NonNull;
use std::{slice, str};

use crate::sqlite::SqliteType;

use super::owned_row::OwnedSqliteRow;
use super::row::PrivateSqliteRow;

/// Raw sqlite value as received from the database
///
/// Use the `read_*` functions to access the actual
/// value or use existing `FromSql` implementations
/// to convert this into rust values
#[allow(missing_debug_implementations, missing_copy_implementations)]
pub struct SqliteValue<'row, 'stmt, 'query> {
    // This field exists to ensure that nobody
    // can modify the underlying row while we are
    // holding a reference to some row value here
    _row: Option<Ref<'row, PrivateSqliteRow<'stmt, 'query>>>,
    // we extract the raw value pointer as part of the constructor
    // to safe the match statements for each method
    // According to benchmarks this leads to a ~20-30% speedup
    //
    // This is sound as long as nobody calls `stmt.step()`
    // while holding this value. We ensure this by including
    // a reference to the row above.
    value: NonNull<ffi::sqlite3_value>,
}

#[derive(Debug)]
#[repr(transparent)]
pub(super) struct OwnedSqliteValue {
    pub(super) value: NonNull<ffi::sqlite3_value>,
}

impl Drop for OwnedSqliteValue {
    fn drop(&mut self) {
        unsafe { ffi::sqlite3_value_free(self.value.as_ptr()) }
    }
}

// Unsafe Send impl safe since sqlite3_value is built with sqlite3_value_dup
// see https://www.sqlite.org/c3ref/value.html
unsafe impl Send for OwnedSqliteValue {}

impl<'row, 'stmt, 'query> SqliteValue<'row, 'stmt, 'query> {
    pub(super) fn new(
        row: Ref<'row, PrivateSqliteRow<'stmt, 'query>>,
        col_idx: usize,
    ) -> Option<SqliteValue<'row, 'stmt, 'query>> {
        let value = match &*row {
            PrivateSqliteRow::Direct(stmt) => stmt.column_value(
                col_idx
                    .try_into()
                    .expect("Diesel expects to run at least on a 32 bit platform"),
            )?,
            PrivateSqliteRow::Duplicated { values, .. } => {
                values.get(col_idx).and_then(|v| v.as_ref())?.value
            }
        };

        let ret = Self {
            _row: Some(row),
            value,
        };
        if ret.value_type().is_none() {
            None
        } else {
            Some(ret)
        }
    }

    pub(super) fn from_owned_row(
        row: &'row OwnedSqliteRow,
        col_idx: usize,
    ) -> Option<SqliteValue<'row, 'stmt, 'query>> {
        let value = row.values.get(col_idx).and_then(|v| v.as_ref())?.value;
        let ret = Self { _row: None, value };
        if ret.value_type().is_none() {
            None
        } else {
            Some(ret)
        }
    }

    pub(crate) fn parse_string<'value, R>(&'value mut self, f: impl FnOnce(&'value str) -> R) -> R {
        let s = unsafe {
            let ptr = ffi::sqlite3_value_text(self.value.as_ptr());
            let len = ffi::sqlite3_value_bytes(self.value.as_ptr());
            let bytes = slice::from_raw_parts(
                ptr,
                len.try_into()
                    .expect("Diesel expects to run at least on a 32 bit platform"),
            );
            // The string is guaranteed to be utf8 according to
            // https://www.sqlite.org/c3ref/value_blob.html
            str::from_utf8_unchecked(bytes)
        };
        f(s)
    }

    /// Read the underlying value as string
    ///
    /// If the underlying value is not a string sqlite will convert it
    /// into a string and return that value instead.
    ///
    /// Use the [`value_type()`](Self::value_type()) function to determine the actual
    /// type of the value.
    ///
    /// See <https://www.sqlite.org/c3ref/value_blob.html> for details
    pub fn read_text(&mut self) -> &str {
        self.parse_string(|s| s)
    }

    /// Read the underlying value as blob
    ///
    /// If the underlying value is not a blob sqlite will convert it
    /// into a blob and return that value instead.
    ///
    /// Use the [`value_type()`](Self::value_type()) function to determine the actual
    /// type of the value.
    ///
    /// See <https://www.sqlite.org/c3ref/value_blob.html> for details
    pub fn read_blob(&mut self) -> &[u8] {
        unsafe {
            let ptr = ffi::sqlite3_value_blob(self.value.as_ptr());
            let len = ffi::sqlite3_value_bytes(self.value.as_ptr());
            if len == 0 {
                // rusts std-lib has an debug_assert that prevents creating
                // slices without elements from a pointer
                &[]
            } else {
                slice::from_raw_parts(
                    ptr as *const u8,
                    len.try_into()
                        .expect("Diesel expects to run at least on a 32 bit platform"),
                )
            }
        }
    }

    /// Read the underlying value as 32 bit integer
    ///
    /// If the underlying value is not an integer sqlite will convert it
    /// into an integer and return that value instead.
    ///
    /// Use the [`value_type()`](Self::value_type()) function to determine the actual
    /// type of the value.
    ///
    /// See <https://www.sqlite.org/c3ref/value_blob.html> for details
    pub fn read_integer(&mut self) -> i32 {
        unsafe { ffi::sqlite3_value_int(self.value.as_ptr()) }
    }

    /// Read the underlying value as 64 bit integer
    ///
    /// If the underlying value is not a string sqlite will convert it
    /// into a string and return that value instead.
    ///
    /// Use the [`value_type()`](Self::value_type()) function to determine the actual
    /// type of the value.
    ///
    /// See <https://www.sqlite.org/c3ref/value_blob.html> for details
    pub fn read_long(&mut self) -> i64 {
        unsafe { ffi::sqlite3_value_int64(self.value.as_ptr()) }
    }

    /// Read the underlying value as 64 bit float
    ///
    /// If the underlying value is not a string sqlite will convert it
    /// into a string and return that value instead.
    ///
    /// Use the [`value_type()`](Self::value_type()) function to determine the actual
    /// type of the value.
    ///
    /// See <https://www.sqlite.org/c3ref/value_blob.html> for details
    pub fn read_double(&mut self) -> f64 {
        unsafe { ffi::sqlite3_value_double(self.value.as_ptr()) }
    }

    /// Get the type of the value as returned by sqlite
    pub fn value_type(&self) -> Option<SqliteType> {
        let tpe = unsafe { ffi::sqlite3_value_type(self.value.as_ptr()) };
        match tpe {
            ffi::SQLITE_TEXT => Some(SqliteType::Text),
            ffi::SQLITE_INTEGER => Some(SqliteType::Long),
            ffi::SQLITE_FLOAT => Some(SqliteType::Double),
            ffi::SQLITE_BLOB => Some(SqliteType::Binary),
            ffi::SQLITE_NULL => None,
            _ => unreachable!(
                "Sqlite's documentation state that this case ({}) is not reachable. \
                 If you ever see this error message please open an issue at \
                 https://github.com/diesel-rs/diesel.",
                tpe
            ),
        }
    }
}

impl OwnedSqliteValue {
    pub(super) fn copy_from_ptr(ptr: NonNull<ffi::sqlite3_value>) -> Option<OwnedSqliteValue> {
        let tpe = unsafe { ffi::sqlite3_value_type(ptr.as_ptr()) };
        if ffi::SQLITE_NULL == tpe {
            return None;
        }
        let value = unsafe { ffi::sqlite3_value_dup(ptr.as_ptr()) };
        Some(Self {
            value: NonNull::new(value)?,
        })
    }

    pub(super) fn duplicate(&self) -> OwnedSqliteValue {
        // self.value is a `NonNull` ptr so this cannot be null
        let value = unsafe { ffi::sqlite3_value_dup(self.value.as_ptr()) };
        let value = NonNull::new(value).expect(
            "Sqlite documentation states this returns only null if value is null \
                 or OOM. If you ever see this panic message please open an issue at \
                 https://github.com/diesel-rs/diesel.",
        );
        OwnedSqliteValue { value }
    }
}

#[cfg(test)]
mod tests {
    use crate::connection::{LoadConnection, SimpleConnection};
    use crate::row::Field;
    use crate::row::Row;
    use crate::sql_types::{Blob, Double, Int4, Text};
    use crate::*;

    #[expect(clippy::approx_constant)] // we really want to use 3.14
    #[diesel_test_helper::test]
    fn can_convert_all_values() {
        let mut conn = SqliteConnection::establish(":memory:").unwrap();

        conn.batch_execute("CREATE TABLE tests(int INTEGER, text TEXT, blob BLOB, float FLOAT)")
            .unwrap();

        diesel::sql_query("INSERT INTO tests(int, text, blob, float) VALUES(?, ?, ?, ?)")
            .bind::<Int4, _>(42)
            .bind::<Text, _>("foo")
            .bind::<Blob, _>(b"foo")
            .bind::<Double, _>(3.14)
            .execute(&mut conn)
            .unwrap();

        let mut res = conn
            .load(diesel::sql_query(
                "SELECT int, text, blob, float FROM tests",
            ))
            .unwrap();
        let row = res.next().unwrap().unwrap();
        let int_field = row.get(0).unwrap();
        let text_field = row.get(1).unwrap();
        let blob_field = row.get(2).unwrap();
        let float_field = row.get(3).unwrap();

        let mut int_value = int_field.value().unwrap();
        assert_eq!(int_value.read_integer(), 42);
        let mut int_value = int_field.value().unwrap();
        assert_eq!(int_value.read_long(), 42);
        let mut int_value = int_field.value().unwrap();
        assert_eq!(int_value.read_double(), 42.0);
        let mut int_value = int_field.value().unwrap();
        assert_eq!(int_value.read_text(), "42");
        let mut int_value = int_field.value().unwrap();
        assert_eq!(int_value.read_blob(), b"42");

        let mut text_value = text_field.value().unwrap();
        assert_eq!(text_value.read_integer(), 0);
        let mut text_value = text_field.value().unwrap();
        assert_eq!(text_value.read_long(), 0);
        let mut text_value = text_field.value().unwrap();
        assert_eq!(text_value.read_double(), 0.0);
        let mut text_value = text_field.value().unwrap();
        assert_eq!(text_value.read_text(), "foo");
        let mut text_value = text_field.value().unwrap();
        assert_eq!(text_value.read_blob(), b"foo");

        let mut blob_value = blob_field.value().unwrap();
        assert_eq!(blob_value.read_integer(), 0);
        let mut blob_value = blob_field.value().unwrap();
        assert_eq!(blob_value.read_long(), 0);
        let mut blob_value = blob_field.value().unwrap();
        assert_eq!(blob_value.read_double(), 0.0);
        let mut blob_value = blob_field.value().unwrap();
        assert_eq!(blob_value.read_text(), "foo");
        let mut blob_value = blob_field.value().unwrap();
        assert_eq!(blob_value.read_blob(), b"foo");

        let mut float_value = float_field.value().unwrap();
        assert_eq!(float_value.read_integer(), 3);
        let mut float_value = float_field.value().unwrap();
        assert_eq!(float_value.read_long(), 3);
        let mut float_value = float_field.value().unwrap();
        assert_eq!(float_value.read_double(), 3.14);
        let mut float_value = float_field.value().unwrap();
        assert_eq!(float_value.read_text(), "3.14");
        let mut float_value = float_field.value().unwrap();
        assert_eq!(float_value.read_blob(), b"3.14");
    }
}
