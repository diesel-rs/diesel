use crate::backend::Backend;
use crate::query_builder::returning_clause::ReturningClause;
use crate::query_builder::{AstPass, QueryFragment};
use crate::result::QueryResult;
use crate::sqlite::backend::SqliteReturningClause;

impl<Expr, DB> QueryFragment<DB, SqliteReturningClause> for ReturningClause<Expr>
where
    DB: Backend<ReturningClause = SqliteReturningClause>,
    Expr: QueryFragment<DB>,
{
    fn walk_ast<'b>(&'b self, mut out: AstPass<'_, 'b, DB>) -> QueryResult<()> {
        out.skip_from(true);
        out.push_sql(" RETURNING ");
        self.0.walk_ast(out.reborrow())?;
        Ok(())
    }
}
