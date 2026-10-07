use crate::QueryResult;
use crate::backend::Backend;
use crate::query_builder::returning::ReturningClause;
use crate::query_builder::{AstPass, QueryFragment};

impl<Expr, DB> QueryFragment<DB, crate::mariadb::backend::MariadbReturningClause>
    for ReturningClause<Expr>
where
    DB: Backend<ReturningClause = crate::mariadb::backend::MariadbReturningClause>,
    Expr: QueryFragment<DB>,
{
    fn walk_ast<'b>(&'b self, mut out: AstPass<'_, 'b, DB>) -> QueryResult<()> {
        out.push_sql(" RETURNING ");
        self.0.walk_ast(out.reborrow())?;
        Ok(())
    }
}
