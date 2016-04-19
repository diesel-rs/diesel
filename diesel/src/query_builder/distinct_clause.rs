use backend::Backend;
use query_builder::*;
use result::QueryResult;

#[derive(Debug, Clone, Copy)]
pub struct NoDistinctClause;
#[derive(Debug, Clone, Copy)]
pub struct DistinctClause;

impl<DB: Backend> QueryFragment<DB> for NoDistinctClause {
    fn to_sql(&self, _out: &mut DB::QueryBuilder) -> BuildQueryResult {
        Ok(())
    }

    fn collect_binds(&self, _out: &mut DB::BindCollector) -> QueryResult<()> {
        Ok(())
    }

    fn is_safe_to_cache_prepared(&self) -> bool {
        true
    }
}

impl_query_id!(NoDistinctClause);

impl<DB: Backend> QueryFragment<DB> for DistinctClause {
    fn to_sql(&self, out: &mut DB::QueryBuilder) -> BuildQueryResult {
        out.push_sql("DISTINCT ");
        Ok(())
    }

    fn collect_binds(&self, _out: &mut DB::BindCollector) -> QueryResult<()> {
        Ok(())
    }

    fn is_safe_to_cache_prepared(&self) -> bool {
        true
    }
}

impl_query_id!(DistinctClause);
