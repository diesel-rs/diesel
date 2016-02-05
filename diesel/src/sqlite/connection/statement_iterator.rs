use std::marker::PhantomData;

use query_source::Queryable;
use result::Error::DeserializationError;
use result::QueryResult;
use sqlite::Sqlite;
use super::stmt::Statement;
use types::{HasSqlType, FromSqlRow};

pub struct StatementIterator<ST, T> {
    stmt: Statement,
    _marker: PhantomData<(ST, T)>,
}

impl<ST, T> StatementIterator<ST, T> {
    pub fn new(stmt: Statement) -> Self {
        StatementIterator {
            stmt: stmt,
            _marker: PhantomData,
        }
    }
}

impl<ST, T> Iterator for StatementIterator<ST, T> where
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
