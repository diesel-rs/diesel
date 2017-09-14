use backend::Backend;
use query_builder::{AstPass, QueryFragment};
use result::QueryResult;

#[derive(Debug, Clone, Copy)]
pub struct NoForUpdateClause;

impl<DB: Backend> QueryFragment<DB> for NoForUpdateClause {
    fn walk_ast(&self, _: AstPass<DB>) -> QueryResult<()> {
        Ok(())
    }
}

impl_query_id!(NoForUpdateClause);

#[derive(Debug, Clone, Copy)]
pub struct ForUpdateClause;

impl_query_id!(ForUpdateClause);
