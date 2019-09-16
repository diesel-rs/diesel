use super::cursor::NamedCursor;
use super::result::PgResult;
use pg::{Pg, PgMetadataLookup, PgValue};
use row::*;

pub struct PgRow<'a> {
    db_result: &'a PgResult<'a>,
    row_idx: usize,
    col_idx: usize,
}

impl<'a> PgRow<'a> {
    pub fn new(db_result: &'a PgResult, row_idx: usize) -> Self {
        PgRow {
            db_result,
            row_idx,
            col_idx: 0,
        }
    }
}

impl<'a> Row<Pg> for PgRow<'a> {
    fn take(&mut self) -> Option<PgValue> {
        let current_idx = self.col_idx;
        self.col_idx += 1;
        let raw = self.db_result.get(self.row_idx, current_idx)?;

        Some(PgValue::new(
            raw,
            self.db_result.column_type(current_idx),
            PgMetadataLookup::new(self.db_result.connection),
        ))
    }

    fn next_is_null(&self, count: usize) -> bool {
        (0..count).all(|i| self.db_result.is_null(self.row_idx, self.col_idx + i))
    }
}

pub struct PgNamedRow<'a> {
    cursor: &'a NamedCursor<'a>,
    idx: usize,
}

impl<'a> PgNamedRow<'a> {
    pub fn new(cursor: &'a NamedCursor, idx: usize) -> Self {
        PgNamedRow { cursor, idx }
    }
}

impl<'a> NamedRow<Pg> for PgNamedRow<'a> {
    fn get_raw_value(&self, index: usize) -> Option<PgValue> {
        let raw = self.cursor.get_value(self.idx, index)?;
        Some(PgValue::new(
            raw,
            self.cursor.db_result.column_type(index),
            PgMetadataLookup::new(self.cursor.db_result.connection),
        ))
    }

    fn index_of(&self, column_name: &str) -> Option<usize> {
        self.cursor.index_of_column(column_name)
    }
}
