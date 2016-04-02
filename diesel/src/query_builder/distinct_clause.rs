use backend::Backend;
use query_builder::{QueryFragment, QueryBuilder, BuildQueryResult};

#[derive(Debug, Clone, Copy)]
pub struct NoDistinctClause;
#[derive(Debug, Clone, Copy)]
pub struct DistinctClause;

impl<DB: Backend> QueryFragment<DB> for NoDistinctClause {
    fn to_sql(&self, _out: &mut DB::QueryBuilder) -> BuildQueryResult {
        Ok(())
    }
}

impl<DB: Backend> QueryFragment<DB> for DistinctClause {
    fn to_sql(&self, out: &mut DB::QueryBuilder) -> BuildQueryResult {
        out.push_sql("DISTINCT ");
        Ok(())
    }
}
