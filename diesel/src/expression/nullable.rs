use expression::{Expression, SelectableExpression, NonAggregate};
use query_builder::{QueryBuilder, BuildQueryResult};
use types::IntoNullable;

pub struct Nullable<T>(T);

impl<T> Nullable<T> {
    pub fn new(expr: T) -> Self {
        Nullable(expr)
    }
}

impl<T> Expression for Nullable<T> where
    T: Expression,
    <T as Expression>::SqlType: IntoNullable,
{
    type SqlType = <<T as Expression>::SqlType as IntoNullable>::Nullable;

    fn to_sql(&self, out: &mut QueryBuilder) -> BuildQueryResult {
        self.0.to_sql(out)
    }

    fn to_insert_sql(&self, out: &mut QueryBuilder) -> BuildQueryResult {
        self.0.to_insert_sql(out)
    }
}

impl<T, QS> SelectableExpression<QS> for Nullable<T> where
    T: SelectableExpression<QS>,
    Nullable<T>: Expression,
{
}

impl<T> NonAggregate for Nullable<T> where
    T: NonAggregate,
    Nullable<T>: Expression,
{
}
