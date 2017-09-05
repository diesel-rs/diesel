use backend::Backend;
use expression::*;
use expression::operators::And;
use expression_methods::*;
use result::QueryResult;
use super::*;
use types::Bool;

pub trait WhereAnd<Predicate> {
    type Output;

    fn and(self, predicate: Predicate) -> Self::Output;
}

#[derive(Debug, Clone, Copy)]
pub struct NoWhereClause;

impl_query_id!(NoWhereClause);

impl<DB: Backend> QueryFragment<DB> for NoWhereClause {
    fn walk_ast(&self, _: AstPass<DB>) -> QueryResult<()> {
        Ok(())
    }
}

impl<Predicate> WhereAnd<Predicate> for NoWhereClause
where
    Predicate: Expression<SqlType = Bool>,
{
    type Output = WhereClause<Predicate>;

    fn and(self, predicate: Predicate) -> Self::Output {
        WhereClause(predicate)
    }
}

impl<DB: Backend> Into<Option<Box<QueryFragment<DB>>>> for NoWhereClause {
    fn into(self) -> Option<Box<QueryFragment<DB>>> {
        None
    }
}

#[derive(Debug, Clone, Copy)]
pub struct WhereClause<Expr>(Expr);

impl<DB, Expr> QueryFragment<DB> for WhereClause<Expr>
where
    DB: Backend,
    Expr: QueryFragment<DB>,
{
    fn walk_ast(&self, mut out: AstPass<DB>) -> QueryResult<()> {
        out.push_sql(" WHERE ");
        self.0.walk_ast(out.reborrow())?;
        Ok(())
    }
}

impl_query_id!(WhereClause<T>);

impl<Expr, Predicate> WhereAnd<Predicate> for WhereClause<Expr>
where
    Expr: Expression<SqlType = Bool>,
    Predicate: Expression<SqlType = Bool>,
{
    type Output = WhereClause<And<Expr, Predicate>>;

    fn and(self, predicate: Predicate) -> Self::Output {
        WhereClause(self.0.and(predicate))
    }
}

impl<'a, DB, Predicate> Into<Option<Box<QueryFragment<DB> + 'a>>> for WhereClause<Predicate>
where
    DB: Backend,
    Predicate: QueryFragment<DB> + 'a,
{
    fn into(self) -> Option<Box<QueryFragment<DB> + 'a>> {
        Some(Box::new(self.0))
    }
}
