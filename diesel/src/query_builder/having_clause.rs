use super::*;
use crate::backend::Backend;
use crate::result::QueryResult;

/// Represents that a query has no `HAVING` clause.
#[derive(Debug, Clone, Copy, QueryId)]
pub struct NoHavingClause;

impl<DB: Backend> QueryFragment<DB> for NoHavingClause {
    fn walk_ast(&self, _: AstPass<DB>) -> QueryResult<()> {
        Ok(())
    }
}
