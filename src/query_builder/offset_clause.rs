use expression::Expression;
use super::{QueryFragment, QueryBuilder, BuildQueryResult};
use types::BigInt;

#[derive(Debug, Clone, Copy)]
pub struct NoOffsetClause;

impl QueryFragment for NoOffsetClause {
    fn to_sql<T: QueryBuilder>(&self, _out: &mut T) -> BuildQueryResult {
        Ok(())
    }
}

#[derive(Debug, Clone, Copy)]
pub struct OffsetClause<Expr>(pub Expr);

impl<Expr: Expression<SqlType=BigInt>> QueryFragment for OffsetClause<Expr> {
    fn to_sql<T: QueryBuilder>(&self, out: &mut T) -> BuildQueryResult {
        out.push_sql(" OFFSET ");
        self.0.to_sql(out)
    }
}
