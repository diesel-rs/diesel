use crate::backend::Backend;
use crate::expression::*;
use crate::query_builder::*;
use crate::query_source::Table;
use crate::result::QueryResult;
use crate::sql_types::IntoNullable;

#[derive(Debug, Copy, Clone, DieselNumericOps)]
pub struct Nullable<T>(T);

impl<T> Nullable<T> {
    pub fn new(expr: T) -> Self {
        Nullable(expr)
    }
}

impl<T> Expression for Nullable<T>
where
    T: Expression,
    <T as Expression>::SqlType: IntoNullable,
{
    type SqlType = <<T as Expression>::SqlType as IntoNullable>::Nullable;
}

impl<T, DB> QueryFragment<DB> for Nullable<T>
where
    DB: Backend,
    T: QueryFragment<DB>,
{
    fn walk_ast(&self, pass: AstPass<DB>) -> QueryResult<()> {
        self.0.walk_ast(pass)
    }
}

/// Nullable can be used in where clauses everywhere, but can only be used in
/// select clauses for outer joins.
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

impl<T> NonAggregate for Nullable<T>
where
    T: NonAggregate,
    Nullable<T>: Expression,
{
}

impl<T, QS> SelectableExpression<QS> for Nullable<T>
where
    Self: AppearsOnTable<QS>,
    T: SelectableExpression<QS>,
    QS: Table,
{
}
