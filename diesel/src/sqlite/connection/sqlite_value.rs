extern crate libsqlite3_sys as ffi;

use std::marker::PhantomData;
use std::ptr::NonNull;
use std::{slice, str};

use crate::row::*;
use crate::sqlite::{Sqlite, SqliteType};

use super::stmt::StatementUse;

/// Raw sqlite value as received from the database
///
/// Use existing `FromSql` implementations to convert this into
/// rust values:
#[allow(missing_debug_implementations, missing_copy_implementations)]
pub struct SqliteValue<'a> {
    value: NonNull<ffi::sqlite3_value>,
    p: PhantomData<&'a ()>,
}

pub struct SqliteRow<'a: 'b, 'b: 'c, 'c> {
    stmt: &'c StatementUse<'a, 'b>,
}

impl<'a> SqliteValue<'a> {
    pub(crate) unsafe fn new(inner: *mut ffi::sqlite3_value) -> Option<Self> {
        NonNull::new(inner)
            .map(|value| SqliteValue {
                value,
                p: PhantomData,
            })
            .and_then(|value| {
                // We check here that the actual value represented by the inner
                // `sqlite3_value` is not `NULL` (is sql meaning, not ptr meaning)
                if value.is_null() {
                    None
                } else {
                    Some(value)
                }
            })
    }

    pub(crate) fn read_text(&self) -> &str {
        unsafe {
            let ptr = ffi::sqlite3_value_text(self.value.as_ptr());
            let len = ffi::sqlite3_value_bytes(self.value.as_ptr());
            let bytes = slice::from_raw_parts(ptr as *const u8, len as usize);
            // The string is guaranteed to be utf8 according to
            // https://www.sqlite.org/c3ref/value_blob.html
            str::from_utf8_unchecked(bytes)
        }
    }

    pub(crate) fn read_blob(&self) -> &[u8] {
        unsafe {
            let ptr = ffi::sqlite3_value_blob(self.value.as_ptr());
            let len = ffi::sqlite3_value_bytes(self.value.as_ptr());
            slice::from_raw_parts(ptr as *const u8, len as usize)
        }
    }

    pub(crate) fn read_integer(&self) -> i32 {
        unsafe { ffi::sqlite3_value_int(self.value.as_ptr()) as i32 }
    }

    pub(crate) fn read_long(&self) -> i64 {
        unsafe { ffi::sqlite3_value_int64(self.value.as_ptr()) as i64 }
    }

    pub(crate) fn read_double(&self) -> f64 {
        unsafe { ffi::sqlite3_value_double(self.value.as_ptr()) as f64 }
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
            _ => unreachable!("Sqlite docs saying this is not reachable"),
        }
    }

    pub(crate) fn is_null(&self) -> bool {
        self.value_type().is_none()
    }
}

impl<'a: 'b, 'b: 'c, 'c> SqliteRow<'a, 'b, 'c> {
    pub(crate) fn new(inner_statement: &'c StatementUse<'a, 'b>) -> Self {
        SqliteRow {
            stmt: inner_statement,
        }
    }
}

impl<'a: 'b, 'b: 'c, 'c> Row<'c, Sqlite> for SqliteRow<'a, 'b, 'c> {
    type Field = SqliteField<'a, 'b, 'c>;
    type InnerPartialRow = Self;

    fn field_count(&self) -> usize {
        self.stmt.column_count() as usize
    }

    fn get<I>(&self, idx: I) -> Option<Self::Field>
    where
        Self: RowIndex<I>,
    {
        let idx = self.idx(idx)?;
        Some(SqliteField {
            stmt: &self.stmt,
            col_idx: idx as i32,
        })
    }

    fn partial_row(&self, range: std::ops::Range<usize>) -> PartialRow<Self::InnerPartialRow> {
        PartialRow::new(self, range)
    }
}

impl<'a: 'b, 'b: 'c, 'c> RowIndex<usize> for SqliteRow<'a, 'b, 'c> {
    fn idx(&self, idx: usize) -> Option<usize> {
        if idx < self.stmt.column_count() as usize {
            Some(idx)
        } else {
            None
        }
    }
}

impl<'a: 'b, 'b: 'c, 'c, 'd> RowIndex<&'d str> for SqliteRow<'a, 'b, 'c> {
    fn idx(&self, field_name: &'d str) -> Option<usize> {
        self.stmt.index_for_column_name(field_name)
    }
}

pub struct SqliteField<'a: 'b, 'b: 'c, 'c> {
    stmt: &'c StatementUse<'a, 'b>,
    col_idx: i32,
}

impl<'a: 'b, 'b: 'c, 'c> Field<'c, Sqlite> for SqliteField<'a, 'b, 'c> {
    fn field_name(&self) -> Option<&'c str> {
        self.stmt.field_name(self.col_idx)
    }

    fn is_null(&self) -> bool {
        self.value().is_none()
    }

    fn value(&self) -> Option<crate::backend::RawValue<'c, Sqlite>> {
        self.stmt.value(self.col_idx)
    }
}
