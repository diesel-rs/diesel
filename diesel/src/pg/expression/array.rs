use std::marker::PhantomData;
use expression::{AppearsOnTable, Expression, IntoSingleTypeExpressionList, SelectableExpression};

#[derive(Debug)]
pub struct Array<T, ST> {
    elements: T,
    _marker: PhantomData<ST>,
}

pub fn array<ST, T>(elements: T) -> Array<T::Expression, ST>
where
    T: IntoSingleTypeExpressionList<ST>,
{
    Array {
        elements: elements.into_single_type_expression_list(),
        _marker: PhantomData,
    }
}

impl<T, ST> Expression for Array<T, ST>
where
    T: Expression,
{
    type SqlType = ::pg::types::sql_types::Array<ST>;
}

use query_builder::{AstPass, QueryFragment};
use backend::Backend;

impl<T, ST, DB> QueryFragment<DB> for Array<T, ST>
where
    DB: Backend,
    for<'a> (&'a T): QueryFragment<DB>,
{
    fn walk_ast(&self, mut out: AstPass<DB>) -> ::result::QueryResult<()> {
        out.push_sql("ARRAY[");
        QueryFragment::walk_ast(&(&self.elements,), out.reborrow())?;
        out.push_sql("]");
        Ok(())
    }
}

impl_query_id!(Array<T, ST>);

impl<T, ST, QS> SelectableExpression<QS> for Array<T, ST>
where
    T: SelectableExpression<QS>,
    Array<T, ST>: AppearsOnTable<QS>,
{
}

impl<T, ST, QS> AppearsOnTable<QS> for Array<T, ST>
where
    T: AppearsOnTable<QS>,
    Array<T, ST>: Expression,
{
}

impl<T, ST> ::expression::NonAggregate for Array<T, ST>
where
    T: ::expression::NonAggregate,
    Array<T, ST>: Expression,
{
}
