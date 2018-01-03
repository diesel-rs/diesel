use backend::Backend;
use query_builder::{AstPass, QueryFragment};
use result::QueryResult;

#[derive(Debug, Clone, Copy, QueryId)]
pub struct NoForUpdateClause;

impl<DB: Backend> QueryFragment<DB> for NoForUpdateClause {
    fn walk_ast(&self, _: AstPass<DB>) -> QueryResult<()> {
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, QueryId)]
pub struct ForUpdateClause;
