use expression::Expression;
use super::{QueryFragment, QueryBuilder, BuildQueryResult};

#[derive(Debug, Clone, Copy)]
pub struct NoOrderClause;

impl QueryFragment for NoOrderClause {
    fn to_sql<T: QueryBuilder>(&self, _out: &mut T) -> BuildQueryResult {
        Ok(())
    }
}

#[derive(Debug, Clone, Copy)]
pub struct OrderClause<Expr>(pub Expr);

impl<Expr: Expression> QueryFragment for OrderClause<Expr> {
    fn to_sql<T: QueryBuilder>(&self, out: &mut T) -> BuildQueryResult {
        out.push_sql(" ORDER BY ");
        self.0.to_sql(out)
    }
}
