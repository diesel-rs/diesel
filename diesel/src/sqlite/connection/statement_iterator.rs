use std::collections::HashMap;
use std::marker::PhantomData;

use query_source::{Queryable, QueryableByName};
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

pub struct NamedStatementIterator<'a, T> {
    stmt: StatementUse<'a>,
    column_indices: HashMap<&'a str, usize>,
    _marker: PhantomData<T>,
}

impl<'a, T> NamedStatementIterator<'a, T> {
    pub fn new(stmt: StatementUse<'a>) -> QueryResult<Self> {
        let column_indices = (0..stmt.num_fields())
            .filter_map(|i| {
                stmt.field_name(i).map(|column| {
                    let column = column.to_str().map_err(|e| DeserializationError(e.into()))?;
                    Ok((column, i))
                })
            })
            .collect::<QueryResult<_>>()?;
        Ok(NamedStatementIterator {
            stmt,
            column_indices,
            _marker: PhantomData,
        })
    }
}

impl<'a, T> Iterator for NamedStatementIterator<'a, T>
where
    T: QueryableByName<Sqlite>,
{
    type Item = QueryResult<T>;

    fn next(&mut self) -> Option<Self::Item> {
        self.stmt.step().map(|row| {
            let row = row.into_named(&self.column_indices);
            T::build(&row).map_err(DeserializationError)
        })
    }
}
