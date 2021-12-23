use crate::backend::Backend;
use crate::query_builder::*;
use crate::query_dsl::order_dsl::ValidOrderingForDistinct;
use crate::result::QueryResult;

#[derive(Debug, Clone, Copy, QueryId)]
pub struct NoDistinctClause;
#[derive(Debug, Clone, Copy, QueryId)]
pub struct DistinctClause;

impl<DB: Backend> QueryFragment<DB> for NoDistinctClause {
    fn walk_ast<'b>(&'b self, _: AstPass<'_, 'b, DB>) -> QueryResult<()> {
        Ok(())
    }
}

impl<DB: Backend> QueryFragment<DB> for DistinctClause {
    fn walk_ast<'b>(&'b self, mut out: AstPass<'_, 'b, DB>) -> QueryResult<()> {
        out.push_sql("DISTINCT ");
        Ok(())
    }
}

impl<O> ValidOrderingForDistinct<NoDistinctClause> for O {}
impl<O> ValidOrderingForDistinct<DistinctClause> for O {}

#[cfg(feature = "postgres")]
pub use crate::pg::DistinctOnClause;
