extern crate libsqlite3_sys as ffi;

use std::cell::Ref;
use std::ptr::NonNull;
use std::{slice, str};

use crate::sqlite::SqliteType;

use super::row::PrivateSqliteRow;

extern "C" {
    pub fn sqlite3_value_free(value: *mut ffi::sqlite3_value);
    pub fn sqlite3_value_dup(value: *const ffi::sqlite3_value) -> *mut ffi::sqlite3_value;
}

/// Raw sqlite value as received from the database
///
/// Use existing `FromSql` implementations to convert this into
/// rust values
#[allow(missing_debug_implementations, missing_copy_implementations)]
pub struct SqliteValue<'a, 'b> {
    row: Ref<'a, PrivateSqliteRow<'b>>,
    col_idx: i32,
}

#[repr(transparent)]
pub struct OwnedSqliteValue {
    pub(super) value: NonNull<ffi::sqlite3_value>,
}

impl Drop for OwnedSqliteValue {
    fn drop(&mut self) {
        unsafe { sqlite3_value_free(self.value.as_ptr()) }
    }
}

impl<'a, 'b> SqliteValue<'a, 'b> {
    pub(super) fn new(row: Ref<'a, PrivateSqliteRow<'b>>, col_idx: i32) -> Option<Self> {
        match &*row {
            PrivateSqliteRow::Direct(stmt) => {
                if stmt.column_type(col_idx).is_none() {
                    return None;
                }
            }
            PrivateSqliteRow::Duplicated { values, .. } => {
                if values
                    .get(col_idx as usize)
                    .and_then(|v| v.as_ref())
                    .is_none()
                {
                    return None;
                }
            }
            PrivateSqliteRow::TemporaryEmpty => todo!(),
        }
        Some(Self { row, col_idx })
    }

    pub(crate) fn parse_string<R>(&self, f: impl FnOnce(&str) -> R) -> R {
        match &*self.row {
            super::row::PrivateSqliteRow::Direct(stmt) => f(stmt.read_column_as_str(self.col_idx)),
            super::row::PrivateSqliteRow::Duplicated { values, .. } => f(values
                .get(self.col_idx as usize)
                .and_then(|o| o.as_ref())
                .expect("We checked that this value is not null")
                .read_as_str()),
            super::row::PrivateSqliteRow::TemporaryEmpty => todo!(),
        }
    }

    pub(crate) fn read_text(&self) -> String {
        self.parse_string(|s| s.to_owned())
    }

    pub(crate) fn read_blob(&self) -> Vec<u8> {
        match &*self.row {
            super::row::PrivateSqliteRow::Direct(stmt) => {
                stmt.read_column_as_blob(self.col_idx).to_owned()
            }
            super::row::PrivateSqliteRow::Duplicated { values, .. } => values
                .get(self.col_idx as usize)
                .and_then(|o| o.as_ref())
                .expect("We checked that this value is not null")
                .read_as_blob()
                .to_owned(),
            super::row::PrivateSqliteRow::TemporaryEmpty => todo!(),
        }
    }

    pub(crate) fn read_integer(&self) -> i32 {
        match &*self.row {
            super::row::PrivateSqliteRow::Direct(stmt) => stmt.read_column_as_integer(self.col_idx),
            super::row::PrivateSqliteRow::Duplicated { values, .. } => values
                .get(self.col_idx as usize)
                .and_then(|o| o.as_ref())
                .expect("We checked that this value is not null")
                .read_as_integer(),
            super::row::PrivateSqliteRow::TemporaryEmpty => todo!(),
        }
    }

    pub(crate) fn read_long(&self) -> i64 {
        match &*self.row {
            super::row::PrivateSqliteRow::Direct(stmt) => stmt.read_column_as_long(self.col_idx),
            super::row::PrivateSqliteRow::Duplicated { values, .. } => values
                .get(self.col_idx as usize)
                .and_then(|o| o.as_ref())
                .expect("We checked that this value is not null")
                .read_as_long(),
            super::row::PrivateSqliteRow::TemporaryEmpty => todo!(),
        }
    }

    pub(crate) fn read_double(&self) -> f64 {
        match &*self.row {
            super::row::PrivateSqliteRow::Direct(stmt) => stmt.read_column_as_double(self.col_idx),
            super::row::PrivateSqliteRow::Duplicated { values, .. } => values
                .get(self.col_idx as usize)
                .and_then(|o| o.as_ref())
                .expect("We checked that this value is not null")
                .read_as_double(),
            super::row::PrivateSqliteRow::TemporaryEmpty => todo!(),
        }
    }

    /// Get the type of the value as returned by sqlite
    pub fn value_type(&self) -> Option<SqliteType> {
        match &*self.row {
            super::row::PrivateSqliteRow::Direct(stmt) => stmt.column_type(self.col_idx),
            super::row::PrivateSqliteRow::Duplicated { values, .. } => values
                .get(self.col_idx as usize)
                .and_then(|o| o.as_ref())
                .expect("We checked that this value is not null")
                .value_type(),
            super::row::PrivateSqliteRow::TemporaryEmpty => todo!(),
        }
    }
}

impl OwnedSqliteValue {
    pub(super) fn copy_from_ptr(ptr: *mut ffi::sqlite3_value) -> Option<OwnedSqliteValue> {
        let tpe = unsafe { ffi::sqlite3_value_type(ptr) };
        if SqliteType::from_raw_sqlite(tpe).is_none() {
            return None;
        }

        let value = unsafe { sqlite3_value_dup(ptr) };

        Some(Self {
            value: NonNull::new(value)?,
        })
    }

    pub(super) fn duplicate(&self) -> OwnedSqliteValue {
        // self.value is a `NonNull` ptr so this cannot be null
        let value = unsafe { sqlite3_value_dup(self.value.as_ptr()) };
        let value = NonNull::new(value).expect(
            "Sqlite documentation states this returns only null if value is null \
                 or OOM. If you ever see this panic message please open an issue at \
                 https://github.com/diesel-rs/diesel.",
        );
        OwnedSqliteValue { value }
    }

    fn read_as_str(&self) -> &str {
        unsafe {
            let ptr = ffi::sqlite3_value_text(self.value.as_ptr());
            let len = ffi::sqlite3_value_bytes(self.value.as_ptr());
            let bytes = slice::from_raw_parts(ptr as *const u8, len as usize);
            // The string is guaranteed to be utf8 according to
            // https://www.sqlite.org/c3ref/value_blob.html
            str::from_utf8_unchecked(bytes)
        }
    }

    fn read_as_blob(&self) -> &[u8] {
        unsafe {
            let ptr = ffi::sqlite3_value_blob(self.value.as_ptr());
            let len = ffi::sqlite3_value_bytes(self.value.as_ptr());
            slice::from_raw_parts(ptr as *const u8, len as usize)
        }
    }

    fn read_as_integer(&self) -> i32 {
        unsafe { ffi::sqlite3_value_int(self.value.as_ptr()) }
    }

    fn read_as_long(&self) -> i64 {
        unsafe { ffi::sqlite3_value_int64(self.value.as_ptr()) }
    }

    fn read_as_double(&self) -> f64 {
        unsafe { ffi::sqlite3_value_double(self.value.as_ptr()) }
    }

    fn value_type(&self) -> Option<SqliteType> {
        let tpe = unsafe { ffi::sqlite3_value_type(self.value.as_ptr()) };
        SqliteType::from_raw_sqlite(tpe)
    }
}
