use super::from_clause::AsQuerySource;
use super::where_clause::{WhereAnd, WhereOr};
use super::*;
use crate::backend::DieselReserveSpecialization;
use crate::expression::grouped::Grouped;
use crate::expression::operators::{And, Or};
use crate::expression::*;
use crate::sql_types::BoolOrNullableBool;

/// Represents that a query has no `HAVING` clause.
#[derive(Debug, Clone, Copy, QueryId)]
pub struct NoHavingClause;

impl<DB> QueryFragment<DB> for NoHavingClause
where
    DB: Backend + DieselReserveSpecialization,
{
    fn walk_ast<'b>(&'b self, _: AstPass<'_, 'b, DB>) -> QueryResult<()> {
        Ok(())
    }
}

impl<Predicate> WhereAnd<Predicate> for NoHavingClause
where
    Predicate: Expression,
    Predicate::SqlType: BoolOrNullableBool,
{
    type Output = HavingClause<Predicate>;

    fn and(self, predicate: Predicate) -> Self::Output {
        HavingClause(predicate)
    }
}

impl<Predicate> WhereOr<Predicate> for NoHavingClause
where
    Predicate: Expression,
    Predicate::SqlType: BoolOrNullableBool,
{
    type Output = HavingClause<Predicate>;

    fn or(self, predicate: Predicate) -> Self::Output {
        HavingClause(predicate)
    }
}

impl<DB> From<NoHavingClause> for BoxedHavingClause<'_, DB> {
    fn from(_: NoHavingClause) -> Self {
        BoxedHavingClause::None
    }
}

/// The `HAVING` clause of a query.
#[derive(Debug, Clone, Copy, QueryId)]
pub struct HavingClause<Expr>(Expr);

impl<DB, Expr> QueryFragment<DB> for HavingClause<Expr>
where
    DB: Backend + DieselReserveSpecialization,
    Expr: QueryFragment<DB>,
{
    fn walk_ast<'b>(&'b self, mut out: AstPass<'_, 'b, DB>) -> QueryResult<()> {
        out.push_sql(" HAVING ");
        self.0.walk_ast(out.reborrow())?;
        Ok(())
    }
}

impl<Expr, Predicate> WhereAnd<Predicate> for HavingClause<Expr>
where
    Expr: Expression,
    Expr::SqlType: BoolOrNullableBool,
    Predicate: Expression,
    Predicate::SqlType: BoolOrNullableBool,
{
    type Output = HavingClause<Grouped<And<Expr, Predicate>>>;

    fn and(self, predicate: Predicate) -> Self::Output {
        HavingClause(Grouped(And::new(self.0, predicate)))
    }
}

impl<Expr, Predicate> WhereOr<Predicate> for HavingClause<Expr>
where
    Expr: Expression,
    Expr::SqlType: BoolOrNullableBool,
    Predicate: Expression,
    Predicate::SqlType: BoolOrNullableBool,
{
    type Output = HavingClause<Grouped<Or<Expr, Predicate>>>;

    fn or(self, predicate: Predicate) -> Self::Output {
        HavingClause(Grouped(Or::new(self.0, predicate)))
    }
}

impl<'a, DB, Predicate> From<HavingClause<Predicate>> for BoxedHavingClause<'a, DB>
where
    DB: Backend,
    Predicate: QueryFragment<DB> + Send + 'a,
{
    fn from(where_clause: HavingClause<Predicate>) -> Self {
        BoxedHavingClause::Having(Box::new(where_clause.0))
    }
}

/// Marker trait indicating that a `HAVING` clause is valid for a given query
/// source.
pub trait ValidHavingClause<QS> {}

impl<QS> ValidHavingClause<QS> for NoHavingClause {}

impl<QS, Expr> ValidHavingClause<QS> for HavingClause<Expr>
where
    Expr: AppearsOnTable<QS::QuerySource>,
    QS: AsQuerySource,
{
}

impl<Expr> ValidHavingClause<NoFromClause> for HavingClause<Expr> where
    Expr: AppearsOnTable<NoFromClause>
{
}

#[allow(missing_debug_implementations)] // We can't...
pub enum BoxedHavingClause<'a, DB> {
    Having(Box<dyn QueryFragment<DB> + Send + 'a>),
    None,
}

impl<DB> QueryFragment<DB> for BoxedHavingClause<'_, DB>
where
    DB: Backend + DieselReserveSpecialization,
{
    fn walk_ast<'b>(&'b self, mut out: AstPass<'_, 'b, DB>) -> QueryResult<()> {
        match *self {
            BoxedHavingClause::Having(ref where_clause) => {
                out.push_sql(" HAVING ");
                where_clause.walk_ast(out)
            }
            BoxedHavingClause::None => Ok(()),
        }
    }
}

impl<DB> QueryId for BoxedHavingClause<'_, DB> {
    type QueryId = ();

    const HAS_STATIC_QUERY_ID: bool = false;
}

impl<'a, DB, Predicate> WhereAnd<Predicate> for BoxedHavingClause<'a, DB>
where
    DB: Backend + 'a,
    Predicate: QueryFragment<DB> + Send + 'a,
    Grouped<And<Box<dyn QueryFragment<DB> + Send + 'a>, Predicate>>: QueryFragment<DB>,
{
    type Output = Self;

    fn and(self, predicate: Predicate) -> Self::Output {
        use self::BoxedHavingClause::Having;

        match self {
            Having(where_clause) => Having(Box::new(Grouped(And::new(where_clause, predicate)))),
            BoxedHavingClause::None => Having(Box::new(predicate)),
        }
    }
}

impl<'a, DB, Predicate> WhereOr<Predicate> for BoxedHavingClause<'a, DB>
where
    DB: Backend + 'a,
    Predicate: QueryFragment<DB> + Send + 'a,
    Grouped<Or<Box<dyn QueryFragment<DB> + Send + 'a>, Predicate>>: QueryFragment<DB>,
{
    type Output = Self;

    fn or(self, predicate: Predicate) -> Self::Output {
        use self::BoxedHavingClause::Having;

        match self {
            Having(where_clause) => Having(Box::new(Grouped(Or::new(where_clause, predicate)))),
            BoxedHavingClause::None => Having(Box::new(predicate)),
        }
    }
}
