use crate::backend::Backend;
use crate::expression::TypedExpressionType;
use crate::expression::*;
use crate::query_builder::*;
use crate::query_source::joins::ToInnerJoin;
use crate::result::QueryResult;
use crate::sql_types::{DieselNumericOps, IntoNullable};

#[derive(Debug, Copy, Clone, DieselNumericOps, ValidGrouping)]
pub struct Nullable<T>(T);

impl<T> Nullable<T> {
    pub fn new(expr: T) -> Self {
        Nullable(expr)
    }
}

impl<T> Expression for Nullable<T>
where
    T: Expression,
    T::SqlType: IntoNullable,
    <T::SqlType as IntoNullable>::Nullable: TypedExpressionType,
{
    type SqlType = <T::SqlType as IntoNullable>::Nullable;
}

impl<T, DB> QueryFragment<DB> for Nullable<T>
where
    DB: Backend,
    T: QueryFragment<DB>,
{
    fn walk_ast<'b>(&'b self, pass: AstPass<'_, 'b, DB>) -> QueryResult<()> {
        self.0.walk_ast(pass)
    }
}

impl<T, QS> AppearsOnTable<QS> for Nullable<T>
where
    T: AppearsOnTable<QS>,
    Nullable<T>: Expression,
{
}

impl<T: QueryId> QueryId for Nullable<T> {
    type QueryId = T::QueryId;

    const HAS_STATIC_QUERY_ID: bool = T::HAS_STATIC_QUERY_ID;
}

impl<T, DB> Selectable<DB> for Option<T>
where
    DB: Backend,
    T: Selectable<DB>,
    Nullable<T::SelectExpression>: Expression,
{
    type SelectExpression = Nullable<T::SelectExpression>;

    fn construct_selection() -> Self::SelectExpression {
        Nullable::new(T::construct_selection())
    }
}

impl<T, QS> SelectableExpression<QS> for Nullable<T>
where
    Self: AppearsOnTable<QS>,
    QS: ToInnerJoin,
    T: SelectableExpression<QS::InnerJoin>,
{
}

impl<T> SelectableExpression<NoFromClause> for Nullable<T> where Self: AppearsOnTable<NoFromClause> {}
