use super::result::PgResult;
use super::row::PgNamedRow;
use deserialize::{FromSqlRow, Queryable, QueryableByName};
use pg::Pg;
use result::Error::DeserializationError;
use result::QueryResult;

use std::marker::PhantomData;

/// The type returned by various [`Connection`](struct.Connection.html) methods.
/// Acts as an iterator over `T`.
pub struct Cursor<'a, ST, T> {
    current_row: usize,
    db_result: PgResult<'a>,
    _marker: PhantomData<(ST, T)>,
}

impl<'a, ST, T> Cursor<'a, ST, T> {
    #[doc(hidden)]
    pub fn new(db_result: PgResult<'a>) -> Self {
        Cursor {
            current_row: 0,
            db_result,
            _marker: PhantomData,
        }
    }
}

impl<'a, ST, T> Iterator for Cursor<'a, ST, T>
where
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

pub struct NamedCursor<'a> {
    pub(crate) db_result: PgResult<'a>,
}

impl<'a> NamedCursor<'a> {
    pub fn new(db_result: PgResult<'a>) -> Self {
        NamedCursor { db_result }
    }

    pub fn collect<T>(self) -> QueryResult<Vec<T>>
    where
        T: QueryableByName<Pg>,
    {
        (0..self.db_result.num_rows())
            .map(|i| {
                let row = PgNamedRow::new(&self, i);
                T::build(&row).map_err(DeserializationError)
            })
            .collect()
    }

    pub fn index_of_column(&self, column_name: &str) -> Option<usize> {
        self.db_result.field_number(column_name)
    }

    pub fn get_value(&self, row: usize, column: usize) -> Option<&[u8]> {
        self.db_result.get(row, column)
    }
}
