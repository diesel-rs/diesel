use db_result::PgResult;

pub trait Row {
    fn take(&mut self) -> Option<&[u8]>;
    fn next_is_null(&self, count: usize) -> bool;
}

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

impl<'a> Row for PgRow<'a> {
    fn take(&mut self) -> Option<&[u8]> {
        let current_idx = self.col_idx;
        self.col_idx += 1;
        self.db_result.get(self.row_idx, current_idx)
    }

    fn next_is_null(&self, count: usize) -> bool {
        (0..count).all(|i| {
            self.db_result.is_null(self.row_idx, self.col_idx + i)
        })
    }
}
