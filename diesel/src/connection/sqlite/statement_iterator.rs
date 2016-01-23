use std::marker::PhantomData;

use backend::Sqlite;
use super::stmt::Statement;
use query_source::Queryable;
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
    type Item = T;

    fn next(&mut self) -> Option<T> {
        self.stmt.step().map(|mut row| {
            let values = match T::Row::build_from_row(&mut row) {
                Ok(value) => value,
                Err(reason) => panic!("Error reading values {}", reason.description()),
            };
            T::build(values)
        })
    }
}
