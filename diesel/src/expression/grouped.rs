use backend::Backend;
use expression::{Expression, NonAggregate};
use query_builder::*;
use result::QueryResult;

#[derive(Debug, Copy, Clone)]
pub struct Grouped<T>(pub T);

impl<T: Expression> Expression for Grouped<T> {
    type SqlType = T::SqlType;
}

impl<T: QueryFragment<DB>, DB: Backend> QueryFragment<DB> for Grouped<T> {
    fn to_sql(&self, out: &mut DB::QueryBuilder) -> BuildQueryResult {
        out.push_sql("(");
        try!(self.0.to_sql(out));
        out.push_sql(")");
        Ok(())
    }

    fn collect_binds(&self, out: &mut DB::BindCollector) -> QueryResult<()> {
        try!(self.0.collect_binds(out));
        Ok(())
    }

    fn is_safe_to_cache_prepared(&self) -> bool {
        self.0.is_safe_to_cache_prepared()
    }
}

impl_query_id!(Grouped<T>);
impl_selectable_expression!(Grouped<T>);

impl<T: NonAggregate> NonAggregate for Grouped<T> where
    Grouped<T>: Expression,
{
}
