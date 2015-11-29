use expression::*;
use expression::expression_methods::*;
use expression::predicates::And;
use super::{QueryFragment, QueryBuilder, BuildQueryResult};
use types::Bool;

pub trait WhereAnd<Predicate: Expression<SqlType=Bool>> {
    type Output: QueryFragment;

    fn and(self, predicate: Predicate) -> Self::Output;
}

#[derive(Debug, Clone, Copy)]
pub struct NoWhereClause;

impl QueryFragment for NoWhereClause {
    fn to_sql(&self, _out: &mut QueryBuilder) -> BuildQueryResult {
        Ok(())
    }
}

impl<Predicate: Expression<SqlType=Bool>> WhereAnd<Predicate> for NoWhereClause {
    type Output = WhereClause<Predicate>;

    fn and(self, predicate: Predicate) -> Self::Output {
        WhereClause(predicate)
    }
}

#[derive(Debug, Clone, Copy)]
pub struct WhereClause<Expr>(Expr);

impl<Expr: Expression<SqlType=Bool>> QueryFragment for WhereClause<Expr> {
    fn to_sql(&self, out: &mut QueryBuilder) -> BuildQueryResult {
        out.push_sql(" WHERE ");
        self.0.to_sql(out)
    }
}

impl<Expr, Predicate> WhereAnd<Predicate> for WhereClause<Expr> where
    Expr: Expression<SqlType=Bool>,
    Predicate: Expression<SqlType=Bool>,
{
    type Output = WhereClause<And<Expr, Predicate>>;

    fn and(self, predicate: Predicate) -> Self::Output {
        WhereClause(self.0.and(predicate))
    }
}
