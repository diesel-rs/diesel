use diesel::backend::Backend;
use diesel::expression::{AppearsOnTable, Expression, SelectableExpression, NonAggregate};
use diesel::prelude::*;
use diesel::query_builder::*;
use std::borrow::Borrow;
use std::marker::PhantomData;

#[derive(Debug, Clone, Copy)]
pub struct Column<T, U, ST> {
    table: T,
    name: U,
    _sql_type: PhantomData<ST>,
}

impl<T, U, ST> Column<T, U, ST> {
    pub(crate) fn new(table: T, name: U) -> Self {
        Self {
            table,
            name: name,
            _sql_type: PhantomData,
        }
    }
}

impl<T, U, ST> QueryId for Column<T, U, ST> {
    type QueryId = ();
    const HAS_STATIC_QUERY_ID: bool = false;
}

impl<T, U, ST, QS> SelectableExpression<QS> for Column<T, U, ST> {
}

impl<T, U, ST, QS> AppearsOnTable<QS> for Column<T, U, ST> {
}

impl<T, U, ST> Expression for Column<T, U, ST> {
    type SqlType = ST;
}

impl<T, U, ST> NonAggregate for Column<T, U, ST> {
}

impl<T, U, ST, DB> QueryFragment<DB> for Column<T, U, ST> 
where
    DB: Backend,
    T: QueryFragment<DB>,
    U: Borrow<str>,
{
    fn walk_ast(&self, mut out: AstPass<DB>) -> QueryResult<()> {
        out.unsafe_to_cache_prepared();
        self.table.walk_ast(out.reborrow())?;
        out.push_sql(".");
        out.push_identifier(self.name.borrow())?;
        Ok(())
    }
}
