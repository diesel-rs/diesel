use crate::backend::Backend;
use crate::query_builder::*;
use crate::query_dsl::order_dsl::ValidOrderingForDistinct;
use crate::result::QueryResult;

#[derive(Debug, Clone, Copy, QueryId)]
pub struct NoDistinctClause;
#[derive(Debug, Clone, Copy, QueryId)]
pub struct DistinctClause;

impl<DB: Backend> QueryFragment<DB> for NoDistinctClause {
    fn walk_ast<'a, 'b>(&'a self, _: AstPass<'_, 'b, DB>) -> QueryResult<()>
    where
        'a: 'b,
    {
        Ok(())
    }
}

impl<DB: Backend> QueryFragment<DB> for DistinctClause {
    fn walk_ast<'a, 'b>(&'a self, mut out: AstPass<'_, 'b, DB>) -> QueryResult<()>
    where
        'a: 'b,
    {
        out.push_sql("DISTINCT ");
        Ok(())
    }
}

impl<O> ValidOrderingForDistinct<NoDistinctClause> for O {}
impl<O> ValidOrderingForDistinct<DistinctClause> for O {}

#[cfg(feature = "postgres")]
pub use crate::pg::DistinctOnClause;
