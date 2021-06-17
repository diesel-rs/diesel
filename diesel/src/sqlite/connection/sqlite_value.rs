extern crate libsqlite3_sys as ffi;

use std::ptr::NonNull;
use std::{slice, str};

use crate::sqlite::SqliteType;

extern "C" {
    pub fn sqlite3_value_free(value: *mut ffi::sqlite3_value);
    pub fn sqlite3_value_dup(value: *const ffi::sqlite3_value) -> *mut ffi::sqlite3_value;
}

/// Raw sqlite value as received from the database
///
/// Use existing `FromSql` implementations to convert this into
/// rust values
#[allow(missing_debug_implementations, missing_copy_implementations)]
#[repr(C)]
pub struct SqliteValue {
    value: ffi::sqlite3_value,
}

pub struct OwnedSqliteValue {
    pub(super) value: NonNull<ffi::sqlite3_value>,
}

impl Drop for OwnedSqliteValue {
    fn drop(&mut self) {
        unsafe { sqlite3_value_free(self.value.as_ptr()) }
    }
}

impl SqliteValue {
    pub(crate) unsafe fn new<'a>(inner: *mut ffi::sqlite3_value) -> Option<&'a Self> {
        let ptr = NonNull::new(inner as *mut SqliteValue)?;
        // This cast is allowed because value is the only field
        // of this struct and this cast is allowed in C + we have a `#[repr(C)]`
        // on this type to fore the layout to be the same
        // (I(weiznich) would like to use `#[repr(transparent)]` here instead, but
        // that does not work as of rust 1.48
        let value = &*ptr.as_ptr();
        // We check if the SQL value is NULL here (in the SQL meaning, not in the ptr meaning)
        if value.is_null() {
            None
        } else {
            Some(value)
        }
    }

    pub(crate) fn read_text(&self) -> &str {
        unsafe {
            let ptr = ffi::sqlite3_value_text(&self.value as *const _ as *mut ffi::sqlite3_value);
            let len = ffi::sqlite3_value_bytes(&self.value as *const _ as *mut ffi::sqlite3_value);
            let bytes = slice::from_raw_parts(ptr as *const u8, len as usize);
            // The string is guaranteed to be utf8 according to
            // https://www.sqlite.org/c3ref/value_blob.html
            str::from_utf8_unchecked(bytes)
        }
    }

    pub(crate) fn read_blob(&self) -> &[u8] {
        unsafe {
            let ptr = ffi::sqlite3_value_blob(&self.value as *const _ as *mut ffi::sqlite3_value);
            let len = ffi::sqlite3_value_bytes(&self.value as *const _ as *mut ffi::sqlite3_value);
            slice::from_raw_parts(ptr as *const u8, len as usize)
        }
    }

    pub(crate) fn read_integer(&self) -> i32 {
        unsafe { ffi::sqlite3_value_int(&self.value as *const _ as *mut ffi::sqlite3_value) as i32 }
    }

    pub(crate) fn read_long(&self) -> i64 {
        unsafe {
            ffi::sqlite3_value_int64(&self.value as *const _ as *mut ffi::sqlite3_value) as i64
        }
    }

    pub(crate) fn read_double(&self) -> f64 {
        unsafe {
            ffi::sqlite3_value_double(&self.value as *const _ as *mut ffi::sqlite3_value) as f64
        }
    }

    /// Get the type of the value as returned by sqlite
    pub fn value_type(&self) -> Option<SqliteType> {
        let tpe =
            unsafe { ffi::sqlite3_value_type(&self.value as *const _ as *mut ffi::sqlite3_value) };
        match tpe {
            ffi::SQLITE_TEXT => Some(SqliteType::Text),
            ffi::SQLITE_INTEGER => Some(SqliteType::Long),
            ffi::SQLITE_FLOAT => Some(SqliteType::Double),
            ffi::SQLITE_BLOB => Some(SqliteType::Binary),
            ffi::SQLITE_NULL => None,
            _ => unreachable!("Sqlite docs saying this is not reachable"),
        }
    }

    pub(crate) fn is_null(&self) -> bool {
        self.value_type().is_none()
    }

    pub(crate) fn duplicate(&self) -> OwnedSqliteValue {
        let value =
            unsafe { sqlite3_value_dup(&self.value as *const _ as *const ffi::sqlite3_value) };
        let value = NonNull::new(value)
            .expect("Sqlite documentation states this returns only null if value is null or OOM");
        OwnedSqliteValue { value }
    }
}

impl OwnedSqliteValue {
    pub(crate) fn duplicate(&self) -> OwnedSqliteValue {
        let value = unsafe { sqlite3_value_dup(self.value.as_ptr()) };
        let value = NonNull::new(value)
            .expect("Sqlite documentation states this returns only null if value is null or OOM");
        OwnedSqliteValue { value }
    }
}
