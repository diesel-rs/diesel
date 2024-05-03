use std::sync::Arc;

use super::sqlite_value::{OwnedSqliteValue, SqliteValue};
use crate::backend::Backend;
use crate::row::{Field, PartialRow, Row, RowIndex, RowSealed};
use crate::sqlite::Sqlite;

#[derive(Debug)]
pub struct OwnedSqliteRow {
    pub(super) values: Vec<Option<OwnedSqliteValue>>,
    column_names: Arc<[Option<String>]>,
}

impl OwnedSqliteRow {
    pub(super) fn new(
        values: Vec<Option<OwnedSqliteValue>>,
        column_names: Arc<[Option<String>]>,
    ) -> Self {
        OwnedSqliteRow {
            values,
            column_names,
        }
    }
}

impl RowSealed for OwnedSqliteRow {}

impl<'a> Row<'a, Sqlite> for OwnedSqliteRow {
    type Field<'field> = OwnedSqliteField<'field> where 'a: 'field, Self: 'field;
    type InnerPartialRow = Self;

    fn field_count(&self) -> usize {
        self.values.len()
    }

    fn get<'field, I>(&'field self, idx: I) -> Option<Self::Field<'field>>
    where
        'a: 'field,
        Self: RowIndex<I>,
    {
        let idx = self.idx(idx)?;
        Some(OwnedSqliteField {
            row: self,
            col_idx: i32::try_from(idx).ok()?,
        })
    }

    fn partial_row(&self, range: std::ops::Range<usize>) -> PartialRow<'_, Self::InnerPartialRow> {
        PartialRow::new(self, range)
    }
}

impl RowIndex<usize> for OwnedSqliteRow {
    fn idx(&self, idx: usize) -> Option<usize> {
        if idx < self.field_count() {
            Some(idx)
        } else {
            None
        }
    }
}

impl<'idx> RowIndex<&'idx str> for OwnedSqliteRow {
    fn idx(&self, field_name: &'idx str) -> Option<usize> {
        self.column_names
            .iter()
            .position(|n| n.as_ref().map(|s| s as &str) == Some(field_name))
    }
}

#[allow(missing_debug_implementations)]
pub struct OwnedSqliteField<'row> {
    pub(super) row: &'row OwnedSqliteRow,
    pub(super) col_idx: i32,
}

impl<'row> Field<'row, Sqlite> for OwnedSqliteField<'row> {
    fn field_name(&self) -> Option<&str> {
        self.row
            .column_names
            .get(self.col_idx as usize)
            .and_then(|o| o.as_ref().map(|s| s.as_ref()))
    }

    fn is_null(&self) -> bool {
        self.value().is_none()
    }

    fn value(&self) -> Option<<Sqlite as Backend>::RawValue<'row>> {
        SqliteValue::from_owned_row(self.row, self.col_idx)
    }
}
