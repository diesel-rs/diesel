use expression::Expression;
use super::{QueryFragment, QueryBuilder, BuildQueryResult};
use types::BigInt;

#[derive(Debug, Clone, Copy)]
pub struct NoLimitClause;

impl QueryFragment for NoLimitClause {
    fn to_sql<T: QueryBuilder>(&self, _out: &mut T) -> BuildQueryResult {
        Ok(())
    }
}

#[derive(Debug, Clone, Copy)]
pub struct LimitClause<Expr>(pub Expr);

impl<Expr: Expression<SqlType=BigInt>> QueryFragment for LimitClause<Expr> {
    fn to_sql<T: QueryBuilder>(&self, out: &mut T) -> BuildQueryResult {
        out.push_sql(" LIMIT ");
        self.0.to_sql(out)
    }
}
