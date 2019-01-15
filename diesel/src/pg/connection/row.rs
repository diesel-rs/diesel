use super::cursor::NamedCursor;
use super::result::PgResult;
use pg::{Pg, PgValue};
use row::*;

pub struct PgRow<'a> {
    db_result: &'a PgResult,
    row_idx: usize,
    col_idx: usize,
}

impl<'a> PgRow<'a> {
    pub fn new(db_result: &'a PgResult, row_idx: usize) -> Self {
        PgRow {
            db_result: db_result,
            row_idx: row_idx,
            col_idx: 0,
        }
    }
}

impl<'a> Row<Pg> for PgRow<'a> {
    fn take(&mut self) -> Option<PgValue> {
        let current_idx = self.col_idx;
        self.col_idx += 1;
        self.db_result.get(self.row_idx, current_idx)
    }

    fn next_is_null(&self, count: usize) -> bool {
        (0..count).all(|i| self.db_result.is_null(self.row_idx, self.col_idx + i))
    }
}

pub struct PgNamedRow<'a> {
    cursor: &'a NamedCursor,
    idx: usize,
}

impl<'a> PgNamedRow<'a> {
    pub fn new(cursor: &'a NamedCursor, idx: usize) -> Self {
        PgNamedRow { cursor, idx }
    }
}

impl<'a> NamedRow<Pg> for PgNamedRow<'a> {
    fn get_raw_value(&self, index: usize) -> Option<PgValue> {
        self.cursor.get_value(self.idx, index)
    }

    fn index_of(&self, column_name: &str) -> Option<usize> {
        self.cursor.index_of_column(column_name)
    }
}
