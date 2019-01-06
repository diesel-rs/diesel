use super::result::PgResult;
use super::row::PgNamedRow;
use crate::deserialize::{FromSqlRow, Queryable, QueryableByName};
use crate::pg::Pg;
use crate::result::Error::DeserializationError;
use crate::result::QueryResult;

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

impl<ST, T> Iterator for Cursor<ST, T>
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

pub struct NamedCursor {
    db_result: PgResult,
}

impl NamedCursor {
    pub fn new(db_result: PgResult) -> Self {
        NamedCursor { db_result }
    }

    pub fn collect<T>(self) -> QueryResult<Vec<T>>
    where
        T: QueryableByName<Pg>,
    {
        use crate::result::Error::DeserializationError;

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
