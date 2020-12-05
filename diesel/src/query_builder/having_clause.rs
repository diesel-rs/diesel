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

/// The `HAVING` clause of a query.
#[derive(Debug, Clone, Copy, QueryId)]
pub struct HavingClause<Expr>(pub Expr);

impl<DB, Expr> QueryFragment<DB> for HavingClause<Expr>
where
    DB: Backend,
    Expr: QueryFragment<DB>,
{
    fn walk_ast(&self, mut out: AstPass<DB>) -> QueryResult<()> {
        out.push_sql(" HAVING ");
        self.0.walk_ast(out.reborrow())?;
        Ok(())
    }
}
