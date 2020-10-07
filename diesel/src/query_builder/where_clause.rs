use super::*;
use crate::backend::Backend;
use crate::expression::grouped::Grouped;
use crate::expression::operators::{And, Or};
use crate::expression::*;
use crate::result::QueryResult;
use crate::sql_types::BoolOrNullableBool;

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
    Predicate: Expression,
    Predicate::SqlType: BoolOrNullableBool,
{
    type Output = WhereClause<Predicate>;

    fn and(self, predicate: Predicate) -> Self::Output {
        WhereClause(predicate)
    }
}

impl<Predicate> WhereOr<Predicate> for NoWhereClause
where
    Predicate: Expression,
    Predicate::SqlType: BoolOrNullableBool,
{
    type Output = WhereClause<Predicate>;

    fn or(self, predicate: Predicate) -> Self::Output {
        WhereClause(predicate)
    }
}

impl<'a, DB> Into<BoxedWhereClause<'a, DB>> for NoWhereClause {
    fn into(self) -> BoxedWhereClause<'a, DB> {
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
    Expr: Expression,
    Expr::SqlType: BoolOrNullableBool,
    Predicate: Expression,
    Predicate::SqlType: BoolOrNullableBool,
{
    type Output = WhereClause<Grouped<And<Expr, Predicate>>>;

    fn and(self, predicate: Predicate) -> Self::Output {
        WhereClause(Grouped(And::new(self.0, predicate)))
    }
}

impl<Expr, Predicate> WhereOr<Predicate> for WhereClause<Expr>
where
    Expr: Expression,
    Expr::SqlType: BoolOrNullableBool,
    Predicate: Expression,
    Predicate::SqlType: BoolOrNullableBool,
{
    type Output = WhereClause<Grouped<Or<Expr, Predicate>>>;

    fn or(self, predicate: Predicate) -> Self::Output {
        WhereClause(Grouped(Or::new(self.0, predicate)))
    }
}

impl<'a, DB, Predicate> Into<BoxedWhereClause<'a, DB>> for WhereClause<Predicate>
where
    DB: Backend,
    Predicate: QueryFragment<DB> + Send + 'a,
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
    Where(Box<dyn QueryFragment<DB> + Send + 'a>),
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
    Predicate: QueryFragment<DB> + Send + 'a,
{
    type Output = Self;

    fn and(self, predicate: Predicate) -> Self::Output {
        use self::BoxedWhereClause::Where;

        match self {
            Where(where_clause) => Where(Box::new(Grouped(And::new(where_clause, predicate)))),
            BoxedWhereClause::None => Where(Box::new(predicate)),
        }
    }
}

impl<'a, DB, Predicate> WhereOr<Predicate> for BoxedWhereClause<'a, DB>
where
    DB: Backend + 'a,
    Predicate: QueryFragment<DB> + Send + 'a,
{
    type Output = Self;

    fn or(self, predicate: Predicate) -> Self::Output {
        use self::BoxedWhereClause::Where;

        match self {
            Where(where_clause) => Where(Box::new(Grouped(Or::new(where_clause, predicate)))),
            BoxedWhereClause::None => Where(Box::new(predicate)),
        }
    }
}
