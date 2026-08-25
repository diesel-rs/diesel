use alloc::vec::Vec;

use core::borrow::Borrow;
use core::marker::PhantomData;
use diesel::backend::Backend;
use diesel::expression::{is_aggregate, TypedExpressionType, ValidGrouping};
use diesel::prelude::*;
use diesel::query_builder::*;
use diesel::query_source::ColumnHasTable;

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, PartialOrd, Ord)]
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

impl<T, S, U, ST, DB> QueryFragment<DB> for Column<crate::Table<T, S>, U, ST>
where
    DB: Backend,
    T: Borrow<str>,
    S: Borrow<str>,
    U: Borrow<str>,
    ST: diesel::sql_types::SqlTypeOrUntyped,
{
    fn walk_ast<'b>(&'b self, mut out: AstPass<'_, 'b, DB>) -> QueryResult<()> {
        out.unsafe_to_cache_prepared();
        self.table.walk_ast(out.reborrow())?;
        out.push_sql(".");
        out.push_identifier(self.name.borrow())?;
        Ok(())
    }

    fn collect_output_metadata<'b>(
        &'b self,
        out: &mut Vec<OutputFieldMetadata<'b>>,
    ) -> QueryResult<()> {
        out.push(OutputFieldMetadata {
            origin: Some(OutputColumnOrigin {
                schema: self.table.schema().map(Borrow::borrow),
                table: self.table.name().borrow(),
                column: self.name.borrow(),
            }),
            declared_sql_type: ST::type_descriptor(),
        });
        Ok(())
    }
}

impl<T, U, ST> ColumnHasTable for Column<T, U, ST>
where
    U: Borrow<str>,
    T: Clone,
{
    type Table = T;

    fn table(&self) -> Self::Table {
        self.table.clone()
    }

    fn name(&self) -> &str {
        self.name.borrow()
    }
}
