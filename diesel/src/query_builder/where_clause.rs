use backend::Backend;
use expression::*;
use expression::expression_methods::*;
use expression::predicates::And;
use super::{QueryFragment, QueryBuilder, BuildQueryResult};
use types::Bool;

pub trait WhereAnd<Predicate> {
    type Output;

    fn and(self, predicate: Predicate) -> Self::Output;
}

#[derive(Debug, Clone, Copy)]
pub struct NoWhereClause;

impl<DB: Backend> QueryFragment<DB> for NoWhereClause {
    fn to_sql(&self, _out: &mut DB::QueryBuilder) -> BuildQueryResult {
        Ok(())
    }
}

impl<Predicate> WhereAnd<Predicate> for NoWhereClause where
    Predicate: Expression<SqlType=Bool>,
{
    type Output = WhereClause<Predicate>;

    fn and(self, predicate: Predicate) -> Self::Output {
        WhereClause(predicate)
    }
}

#[derive(Debug, Clone, Copy)]
pub struct WhereClause<Expr>(Expr);

impl<DB, Expr> QueryFragment<DB> for WhereClause<Expr> where
    DB: Backend,
    Expr: QueryFragment<DB>,
{
    fn to_sql(&self, out: &mut DB::QueryBuilder) -> BuildQueryResult {
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
