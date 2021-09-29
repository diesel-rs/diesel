extern crate libsqlite3_sys as ffi;

use std::cell::Ref;
use std::ptr::NonNull;
use std::{slice, str};

use crate::sqlite::SqliteType;

use super::row::PrivateSqliteRow;

/// Raw sqlite value as received from the database
///
/// Use existing `FromSql` implementations to convert this into
/// rust values
#[allow(missing_debug_implementations, missing_copy_implementations)]
pub struct SqliteValue<'a, 'b> {
    // This field exists to ensure that nobody
    // can modify the underlying row while we are
    // holding a reference to some row value here
    _row: Ref<'a, PrivateSqliteRow<'b>>,
    // we extract the raw value pointer as part of the constructor
    // to safe the match statements for each method
    // Acconding to benchmarks this leads to a ~20-30% speedup
    //
    // This is sound as long as nobody calls `stmt.step()`
    // while holding this value. We ensure this by including
    // a reference to the row above.
    value: NonNull<ffi::sqlite3_value>,
}

#[repr(transparent)]
pub struct OwnedSqliteValue {
    pub(super) value: NonNull<ffi::sqlite3_value>,
}

impl Drop for OwnedSqliteValue {
    fn drop(&mut self) {
        unsafe { ffi::sqlite3_value_free(self.value.as_ptr()) }
    }
}

impl<'a, 'b> SqliteValue<'a, 'b> {
    pub(super) fn new(row: Ref<'a, PrivateSqliteRow<'b>>, col_idx: i32) -> Option<Self> {
        let value = match &*row {
            PrivateSqliteRow::Direct(stmt) => stmt.column_value(col_idx)?,
            PrivateSqliteRow::Duplicated { values, .. } => {
                values.get(col_idx as usize).and_then(|v| v.as_ref())?.value
            }
            PrivateSqliteRow::TemporaryEmpty => {
                // This cannot happen as this is only a temproray state
                // used inside of `StatementIterator::next()`
                unreachable!(
                    "You've reached an impossible internal state. \
                     If you ever see this error message please open \
                     an issue at https://github.com/diesel-rs/diesel \
                     providing example code how to trigger this error."
                )
            }
        };

        let ret = Self { _row: row, value };
        if ret.value_type().is_none() {
            None
        } else {
            Some(ret)
        }
    }

    pub(crate) fn parse_string<'c, R>(&'c self, f: impl FnOnce(&'c str) -> R) -> R {
        let s = unsafe {
            let ptr = ffi::sqlite3_value_text(self.value.as_ptr());
            let len = ffi::sqlite3_value_bytes(self.value.as_ptr());
            let bytes = slice::from_raw_parts(ptr as *const u8, len as usize);
            // The string is guaranteed to be utf8 according to
            // https://www.sqlite.org/c3ref/value_blob.html
            str::from_utf8_unchecked(bytes)
        };
        f(s)
    }

    pub(crate) fn read_text(&self) -> &str {
        self.parse_string(|s| s)
    }

    pub(crate) fn read_blob(&self) -> &[u8] {
        unsafe {
            let ptr = ffi::sqlite3_value_blob(self.value.as_ptr());
            let len = ffi::sqlite3_value_bytes(self.value.as_ptr());
            slice::from_raw_parts(ptr as *const u8, len as usize)
        }
    }

    pub(crate) fn read_integer(&self) -> i32 {
        unsafe { ffi::sqlite3_value_int(self.value.as_ptr()) }
    }

    pub(crate) fn read_long(&self) -> i64 {
        unsafe { ffi::sqlite3_value_int64(self.value.as_ptr()) }
    }

    pub(crate) fn read_double(&self) -> f64 {
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
                 https://github.com/diesel-rs/diesel."
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
