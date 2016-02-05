use pg::Pg;
use query_source::Queryable;
use result::QueryResult;
use result::Error::DeserializationError;
use super::result::PgResult;
use types::{HasSqlType, FromSqlRow};

use std::marker::PhantomData;

/// The type returned by various [`Connection`](struct.Connection.html) methods.
/// Acts as an iterator over `T`.
pub struct Cursor<ST, T> {
    current_row: usize,
    db_result: PgResult,
    _marker: PhantomData<(ST, T)>,
}

impl<ST, T> Cursor<ST, T> {
    #[doc(hidden)]
    pub fn new(db_result: PgResult) -> Self {
        Cursor {
            current_row: 0,
            db_result: db_result,
            _marker: PhantomData,
        }
    }
}

impl<ST, T> Iterator for Cursor<ST, T> where
    Pg: HasSqlType<ST>,
    T: Queryable<ST, Pg>,
{
    type Item = QueryResult<T>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.current_row >= self.db_result.num_rows() {
            None
        } else {
            let mut row = self.db_result.get_row(self.current_row);
            self.current_row += 1;
            let value = T::Row::build_from_row(&mut row)
                .map(T::build)
                .map_err(DeserializationError);
            Some(value)
        }
    }
}
