use crate::backend::Backend;
use crate::query_builder::*;
use crate::query_dsl::order_dsl::ValidOrderingForDistinct;
use crate::result::QueryResult;


#[derive(Debug, Clone, Copy, QueryId)]
///xxxxxxxxxx
pub struct NoDistinctClause;
#[derive(Debug, Clone, Copy, QueryId)]
///xxxxxxxxxxx
pub struct DistinctClause;

impl<DB: Backend> QueryFragment<DB> for NoDistinctClause {
    fn walk_ast(&self, _: AstPass<DB>) -> QueryResult<()> {
        Ok(())
    }
}

impl<DB: Backend> QueryFragment<DB> for DistinctClause {
    fn walk_ast(&self, mut out: AstPass<DB>) -> QueryResult<()> {
        out.push_sql("DISTINCT ");
        Ok(())
    }
}

impl<O> ValidOrderingForDistinct<NoDistinctClause> for O {}
impl<O> ValidOrderingForDistinct<DistinctClause> for O {}

#[cfg(feature = "postgres")]
pub use crate::pg::DistinctOnClause;
