use std::marker::PhantomData;

use super::stmt::StatementUse;
use crate::deserialize::FromSqlRow;
use crate::result::Error::DeserializationError;
use crate::result::QueryResult;
use crate::sqlite::Sqlite;

pub struct StatementIterator<'a: 'b, 'b, ST, T> {
    stmt: StatementUse<'a, 'b>,
    _marker: PhantomData<(ST, T)>,
}

impl<'a: 'b, 'b, ST, T> StatementIterator<'a, 'b, ST, T> {
    pub fn new(stmt: StatementUse<'a, 'b>) -> Self {
        StatementIterator {
            stmt,
            _marker: PhantomData,
        }
    }
}

impl<'a: 'b, 'b, ST, T> Iterator for StatementIterator<'a, 'b, ST, T>
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
