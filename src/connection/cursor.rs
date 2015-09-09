use Queriable;
use db_result::DbResult;
use types::{NativeSqlType, FromSql};

use std::marker::PhantomData;

pub struct Cursor<ST, T> {
    current_row: usize,
    db_result: DbResult,
    _marker: PhantomData<(ST, T)>,
}

impl<ST, T> Cursor<ST, T> {
    pub fn new(db_result: DbResult) -> Self {
        Cursor {
            current_row: 0,
            db_result: db_result,
            _marker: PhantomData,
        }
    }
}

impl<ST, T> Iterator for Cursor<ST, T> where
    ST: NativeSqlType,
    T: Queriable<ST>,
{
    type Item = T;

    fn next(&mut self) -> Option<T> {
        if self.current_row >= self.db_result.num_rows() {
            None
        } else {
            let mut row = self.db_result.get_row(self.current_row);
            self.current_row += 1;
            let values = T::Row::from_sql(&mut row);
            let result = T::build(values);
            Some(result)
        }
    }
}
