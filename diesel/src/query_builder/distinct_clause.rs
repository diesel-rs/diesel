use backend::Backend;
use query_builder::*;
use result::QueryResult;

#[derive(Debug, Clone, Copy)]
pub struct NoDistinctClause;
#[derive(Debug, Clone, Copy)]
pub struct DistinctClause;

impl<DB: Backend> QueryFragment<DB> for NoDistinctClause {
    fn walk_ast(&self, _: AstPass<DB>) -> QueryResult<()> {
        Ok(())
    }
}

impl_query_id!(NoDistinctClause);

impl<DB: Backend> QueryFragment<DB> for DistinctClause {
    fn walk_ast(&self, mut out: AstPass<DB>) -> QueryResult<()> {
        out.push_sql("DISTINCT ");
        Ok(())
    }
}

impl_query_id!(DistinctClause);
