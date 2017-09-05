use std::marker::PhantomData;

use query_source::Queryable;
use result::Error::DeserializationError;
use result::QueryResult;
use sqlite::Sqlite;
use super::stmt::StatementUse;
use types::{FromSqlRow, HasSqlType};

pub struct StatementIterator<'a, ST, T> {
    stmt: StatementUse<'a>,
    _marker: PhantomData<(ST, T)>,
}

impl<'a, ST, T> StatementIterator<'a, ST, T> {
    pub fn new(stmt: StatementUse<'a>) -> Self {
        StatementIterator {
            stmt: stmt,
            _marker: PhantomData,
        }
    }
}

impl<'a, ST, T> Iterator for StatementIterator<'a, ST, T>
where
    Sqlite: HasSqlType<ST>,
    T: Queryable<ST, Sqlite>,
{
    type Item = QueryResult<T>;

    fn next(&mut self) -> Option<Self::Item> {
        self.stmt.step().map(|mut row| {
            T::Row::build_from_row(&mut row)
                .map(T::build)
                .map_err(DeserializationError)
        })
    }
}
