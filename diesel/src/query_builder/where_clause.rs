use super::*;
use crate::backend::Backend;
use crate::dsl::Or;
use crate::expression::operators::And;
use crate::expression::*;
use crate::expression_methods::*;
use crate::result::QueryResult;
use crate::sql_types::Bool;

/// Add `Predicate` to the current `WHERE` clause, joining with `AND` if
/// applicable.
pub trait WhereAnd<Predicate> {
    /// What is the type of the resulting `WHERE` clause?
    type Output;

    /// See the trait-level docs.
    fn and(self, predicate: Predicate) -> Self::Output;
}

/// Add `Predicate` to the current `WHERE` clause, joining with `OR` if
/// applicable.
pub trait WhereOr<Predicate> {
    /// What is the type of the resulting `WHERE` clause?
    type Output;

    /// See the trait-level docs.
    fn or(self, predicate: Predicate) -> Self::Output;
}

/// Represents that a query has no `WHERE` clause.
#[derive(Debug, Clone, Copy, QueryId)]
pub struct NoWhereClause;

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

impl<Predicate> WhereOr<Predicate> for NoWhereClause
where
    Predicate: Expression<SqlType = Bool>,
{
    type Output = WhereClause<Predicate>;

    fn or(self, predicate: Predicate) -> Self::Output {
        WhereClause(predicate)
    }
}

impl<DB> Into<BoxedWhereClause<'static, DB>> for NoWhereClause {
    fn into(self) -> BoxedWhereClause<'static, DB> {
        BoxedWhereClause::None
    }
}

/// The `WHERE` clause of a query.
#[derive(Debug, Clone, Copy, QueryId)]
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

impl<Expr, Predicate> WhereOr<Predicate> for WhereClause<Expr>
where
    Expr: Expression<SqlType = Bool>,
    Predicate: Expression<SqlType = Bool>,
{
    type Output = WhereClause<Or<Expr, Predicate>>;

    fn or(self, predicate: Predicate) -> Self::Output {
        WhereClause(self.0.or(predicate))
    }
}

impl<'a, DB, Predicate> Into<BoxedWhereClause<'a, DB>> for WhereClause<Predicate>
where
    DB: Backend,
    Predicate: QueryFragment<DB> + 'a,
{
    fn into(self) -> BoxedWhereClause<'a, DB> {
        BoxedWhereClause::Where(Box::new(self.0))
    }
}

/// Marker trait indicating that a `WHERE` clause is valid for a given query
/// source.
pub trait ValidWhereClause<QS> {}

impl<QS> ValidWhereClause<QS> for NoWhereClause {}

impl<QS, Expr> ValidWhereClause<QS> for WhereClause<Expr> where Expr: AppearsOnTable<QS> {}

#[allow(missing_debug_implementations)] // We can't...
pub enum BoxedWhereClause<'a, DB> {
    Where(Box<QueryFragment<DB> + 'a>),
    None,
}

impl<'a, DB> QueryFragment<DB> for BoxedWhereClause<'a, DB>
where
    DB: Backend,
{
    fn walk_ast(&self, mut out: AstPass<DB>) -> QueryResult<()> {
        match *self {
            BoxedWhereClause::Where(ref where_clause) => {
                out.push_sql(" WHERE ");
                where_clause.walk_ast(out)
            }
            BoxedWhereClause::None => Ok(()),
        }
    }
}

impl<'a, DB> QueryId for BoxedWhereClause<'a, DB> {
    type QueryId = ();

    const HAS_STATIC_QUERY_ID: bool = false;
}

impl<'a, DB, Predicate> WhereAnd<Predicate> for BoxedWhereClause<'a, DB>
where
    DB: Backend + 'a,
    Predicate: QueryFragment<DB> + 'a,
{
    type Output = Self;

    fn and(self, predicate: Predicate) -> Self::Output {
        use self::BoxedWhereClause::Where;
        use crate::expression::operators::And;

        match self {
            Where(where_clause) => Where(Box::new(And::new(where_clause, predicate))),
            BoxedWhereClause::None => Where(Box::new(predicate)),
        }
    }
}

impl<'a, DB, Predicate> WhereOr<Predicate> for BoxedWhereClause<'a, DB>
where
    DB: Backend + 'a,
    Predicate: QueryFragment<DB> + 'a,
{
    type Output = Self;

    fn or(self, predicate: Predicate) -> Self::Output {
        use self::BoxedWhereClause::Where;
        use crate::expression::grouped::Grouped;
        use crate::expression::operators::Or;

        match self {
            Where(where_clause) => Where(Box::new(Grouped(Or::new(where_clause, predicate)))),
            BoxedWhereClause::None => Where(Box::new(predicate)),
        }
    }
}
