use std::marker::PhantomData;

use super::stmt::StatementUse;
use crate::deserialize::FromSqlRow;
use crate::result::Error::DeserializationError;
use crate::result::QueryResult;
use crate::sqlite::Sqlite;

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
    T: FromSqlRow<ST, Sqlite>,
{
    type Item = QueryResult<T>;

    fn next(&mut self) -> Option<Self::Item> {
        let row = match self.stmt.step() {
            Ok(row) => row,
            Err(e) => return Some(Err(e)),
        };
        row.map(|row| T::build_from_row(&row).map_err(DeserializationError))
    }
}
