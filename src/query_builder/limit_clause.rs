use expression::Expression;
use super::{QueryFragment, QueryBuilder, BuildQueryResult};
use types::BigInt;

#[derive(Debug, Clone, Copy)]
pub struct NoLimitClause;

impl QueryFragment for NoLimitClause {
    fn to_sql(&self, _out: &mut QueryBuilder) -> BuildQueryResult {
        Ok(())
    }
}

#[derive(Debug, Clone, Copy)]
pub struct LimitClause<Expr>(pub Expr);

impl<Expr: Expression<SqlType=BigInt>> QueryFragment for LimitClause<Expr> {
    fn to_sql(&self, out: &mut QueryBuilder) -> BuildQueryResult {
        out.push_sql(" LIMIT ");
        self.0.to_sql(out)
    }
}
