use super::{QueryFragment, QueryBuilder, BuildQueryResult};

#[derive(Debug, Clone, Copy)]
pub struct NoOffsetClause;

impl QueryFragment for NoOffsetClause {
    fn to_sql(&self, _out: &mut QueryBuilder) -> BuildQueryResult {
        Ok(())
    }
}

#[derive(Debug, Clone, Copy)]
pub struct OffsetClause<Expr>(pub Expr);

impl<Expr: QueryFragment> QueryFragment for OffsetClause<Expr> {
    fn to_sql(&self, out: &mut QueryBuilder) -> BuildQueryResult {
        out.push_sql(" OFFSET ");
        self.0.to_sql(out)
    }
}
