use diesel::backend::Backend;
use diesel::expression::{is_aggregate, TypedExpressionType, ValidGrouping};
use diesel::prelude::*;
use diesel::query_builder::*;
use std::borrow::Borrow;
use std::marker::PhantomData;

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
/// A database table column.
/// This type is created by the [`column`](crate::Table::column()) function.
pub struct Column<T, U, ST> {
    table: T,
    name: U,
    _sql_type: PhantomData<ST>,
}

impl<T, U, ST> Column<T, U, ST> {
    pub(crate) fn new(table: T, name: U) -> Self {
        Self {
            table,
            name,
            _sql_type: PhantomData,
        }
    }

    /// Gets a reference to the table of the column.
    pub fn table(&self) -> &T {
        &self.table
    }

    /// Gets the name of the column, as provided on creation.
    pub fn name(&self) -> &U {
        &self.name
    }
}

impl<T, U, ST> QueryId for Column<T, U, ST> {
    type QueryId = ();
    const HAS_STATIC_QUERY_ID: bool = false;
}

impl<T, U, ST, QS> SelectableExpression<QS> for Column<T, U, ST> where Self: Expression {}

impl<T, U, ST, QS> AppearsOnTable<QS> for Column<T, U, ST> where Self: Expression {}

impl<T, U, ST> Expression for Column<T, U, ST>
where
    ST: TypedExpressionType,
{
    type SqlType = ST;
}

impl<T, U, ST> ValidGrouping<()> for Column<T, U, ST> {
    type IsAggregate = is_aggregate::No;
}

impl<T, U, ST, DB> QueryFragment<DB> for Column<T, U, ST>
where
    DB: Backend,
    T: QueryFragment<DB>,
    U: Borrow<str>,
{
    fn walk_ast<'b>(&'b self, mut out: AstPass<'_, 'b, DB>) -> QueryResult<()> {
        out.unsafe_to_cache_prepared();
        self.table.walk_ast(out.reborrow())?;
        out.push_sql(".");
        out.push_identifier(self.name.borrow())?;
        Ok(())
    }
}
