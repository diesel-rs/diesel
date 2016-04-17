use std::marker::PhantomData;

use query_source::Queryable;
use result::Error::DeserializationError;
use result::QueryResult;
use sqlite::Sqlite;
use super::stmt::Statement;
use types::{HasSqlType, FromSqlRow};

pub struct StatementIterator<'a, ST, T> {
    stmt: &'a mut Statement,
    _marker: PhantomData<(ST, T)>,
}

impl<'a, ST, T> StatementIterator<'a, ST, T> {
    pub fn new(stmt: &'a mut Statement) -> Self {
        StatementIterator {
            stmt: stmt,
            _marker: PhantomData,
        }
    }
}

impl<'a, ST, T> Iterator for StatementIterator<'a, ST, T> where
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
