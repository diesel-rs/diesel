use Queriable;
use db_result::DbResult;
use types::{NativeSqlType, FromSqlRow};

use std::marker::PhantomData;

/// The type returned by various [`Connection`](struct.Connection.html) methods.
/// Acts as an iterator over `T`.
pub struct Cursor<ST, T, R> {
    current_row: usize,
    db_result: R,
    _marker: PhantomData<(ST, T)>,
}

impl<ST, T, R> Cursor<ST, T, R> where
    R: DbResult,
{
    #[doc(hidden)]
    pub fn new(db_result: R) -> Self {
        Cursor {
            current_row: 0,
            db_result: db_result,
            _marker: PhantomData,
        }
    }
}

impl<ST, T, R> Iterator for Cursor<ST, T, R> where
    ST: NativeSqlType,
    T: Queriable<ST>,
    R: DbResult,
{
    type Item = T;

    fn next(&mut self) -> Option<T> {
        if self.current_row >= self.db_result.num_rows() {
            None
        } else {
            let mut row = self.db_result.get_row(self.current_row);
            self.current_row += 1;
            let values = match T::Row::build_from_row(&mut row) {
                Ok(value) => value,
                Err(reason) => panic!("Error reading values {}", reason.description()),
            };
            let result = T::build(values);
            Some(result)
        }
    }
}
