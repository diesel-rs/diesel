use backend::Backend;
use expression::{Expression, SelectableExpression, NonAggregate};
use query_builder::*;
use result::QueryResult;
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
}

impl<T, DB> QueryFragment<DB> for Nullable<T> where
    DB: Backend,
    T: QueryFragment<DB>,
{
    fn to_sql(&self, out: &mut DB::QueryBuilder) -> BuildQueryResult {
        self.0.to_sql(out)
    }

    fn collect_binds(&self, out: &mut DB::BindCollector) -> QueryResult<()> {
        self.0.collect_binds(out)
    }

    fn is_safe_to_cache_prepared(&self) -> bool {
        self.0.is_safe_to_cache_prepared()
    }
}

impl<T, QS> SelectableExpression<QS> for Nullable<T> where
    T: SelectableExpression<QS>,
    Nullable<T>: Expression,
{
}

impl<T: QueryId> QueryId for Nullable<T> {
    type QueryId = T::QueryId;

    fn has_static_query_id() -> bool {
        T::has_static_query_id()
    }
}

impl<T> NonAggregate for Nullable<T> where
    T: NonAggregate,
    Nullable<T>: Expression,
{
}
