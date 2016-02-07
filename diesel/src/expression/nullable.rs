use backend::Backend;
use expression::{Expression, NonAggregate, SelectableExpression};
use query_builder::*;
use types::IntoNullable;

pub struct Nullable<T>(T);

impl<T> Nullable<T> {
    pub fn new(expr: T) -> Self {
        Nullable(expr)
    }
}

impl<T> Expression for Nullable<T>
    where T: Expression,
          <T as Expression>::SqlType: IntoNullable,
{
    type SqlType = <<T as Expression>::SqlType as IntoNullable>::Nullable;
}

impl<T, DB> QueryFragment<DB> for Nullable<T>
    where DB: Backend,
          T: QueryFragment<DB>,
{
    fn to_sql(&self, out: &mut DB::QueryBuilder) -> BuildQueryResult {
        self.0.to_sql(out)
    }
}

impl<T, QS> SelectableExpression<QS> for Nullable<T>
    where T: SelectableExpression<QS>,
          Nullable<T>: Expression,
{
}

impl<T> NonAggregate for Nullable<T>
    where T: NonAggregate,
          Nullable<T>: Expression,
{
}
