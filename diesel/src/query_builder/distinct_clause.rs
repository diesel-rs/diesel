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

    fn walk_ast(&self, _: AstPass<DB>) -> QueryResult<()> {
        Ok(())
    }
}

impl_query_id!(NoDistinctClause);

impl<DB: Backend> QueryFragment<DB> for DistinctClause {
    fn to_sql(&self, out: &mut DB::QueryBuilder) -> BuildQueryResult {
        out.push_sql("DISTINCT ");
        Ok(())
    }

    fn walk_ast(&self, _: AstPass<DB>) -> QueryResult<()> {
        Ok(())
    }
}

impl_query_id!(DistinctClause);
