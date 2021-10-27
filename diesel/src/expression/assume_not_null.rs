use crate::backend::Backend;
use crate::expression::TypedExpressionType;
use crate::expression::*;
use crate::query_builder::*;
use crate::query_source::joins::ToInnerJoin;
use crate::result::QueryResult;
use crate::sql_types::{DieselNumericOps, IntoNotNullable};

#[derive(Debug, Copy, Clone, DieselNumericOps, ValidGrouping)]
pub struct AssumeNotNull<T>(T);

impl<T> AssumeNotNull<T> {
    pub fn new(expr: T) -> Self {
        AssumeNotNull(expr)
    }
}

impl<T> Expression for AssumeNotNull<T>
where
    T: Expression,
    T::SqlType: IntoNotNullable,
    <T::SqlType as IntoNotNullable>::NotNullable: TypedExpressionType,
{
    type SqlType = <T::SqlType as IntoNotNullable>::NotNullable;
}

impl<T, DB> QueryFragment<DB> for AssumeNotNull<T>
where
    DB: Backend,
    T: QueryFragment<DB>,
{
    fn walk_ast<'a, 'b>(&'a self, pass: AstPass<'_, 'b, DB>) -> QueryResult<()>
    where
        'a: 'b,
    {
        self.0.walk_ast(pass)
    }
}

impl<T, QS> AppearsOnTable<QS> for AssumeNotNull<T>
where
    T: AppearsOnTable<QS>,
    AssumeNotNull<T>: Expression,
{
}

impl<T: QueryId> QueryId for AssumeNotNull<T> {
    type QueryId = T::QueryId;

    const HAS_STATIC_QUERY_ID: bool = T::HAS_STATIC_QUERY_ID;
}

impl<T, QS> SelectableExpression<QS> for AssumeNotNull<T>
where
    Self: AppearsOnTable<QS>,
    QS: ToInnerJoin,
    T: SelectableExpression<QS::InnerJoin>,
{
}

impl<T> SelectableExpression<NoFromClause> for AssumeNotNull<T> where
    Self: AppearsOnTable<NoFromClause>
{
}
