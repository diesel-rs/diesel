use super::DeleteStatement;
use super::InsertStatement;
use super::UpdateStatement;
use super::{AstPass, QueryFragment};
use crate::backend::{Backend, DieselReserveSpecialization};
use crate::query_builder::QueryId;
use crate::result::QueryResult;
use crate::QuerySource;

#[derive(Debug, Clone, Copy, QueryId)]
pub struct NoReturningClause;

impl<DB> QueryFragment<DB> for NoReturningClause
where
    DB: Backend + DieselReserveSpecialization,
{
    fn walk_ast<'b>(&'b self, _: AstPass<'_, 'b, DB>) -> QueryResult<()> {
        Ok(())
    }
}

/// This type represents a SQL `Returning` clause
///
/// Custom backends can specialize the [`QueryFragment`]
/// implementation via
/// [`SqlDialect::ReturningClause`](crate::backend::SqlDialect::ReturningClause)
#[cfg_attr(
    feature = "i-implement-a-third-party-backend-and-opt-into-breaking-changes",
    cfg(feature = "i-implement-a-third-party-backend-and-opt-into-breaking-changes")
)]
#[derive(Debug, Clone, Copy, QueryId)]
pub struct ReturningClause<Expr>(pub Expr);

impl<Expr, DB> QueryFragment<DB> for ReturningClause<Expr>
where
    DB: Backend,
    Self: QueryFragment<DB, DB::ReturningClause>,
{
    fn walk_ast<'b>(&'b self, pass: AstPass<'_, 'b, DB>) -> QueryResult<()> {
        <Self as QueryFragment<DB, DB::ReturningClause>>::walk_ast(self, pass)
    }
}

impl<Expr, DB>
    QueryFragment<DB, crate::backend::sql_dialect::returning_clause::PgLikeReturningClause>
    for ReturningClause<Expr>
where
    DB: Backend<
        ReturningClause = crate::backend::sql_dialect::returning_clause::PgLikeReturningClause,
    >,
    Expr: QueryFragment<DB>,
{
    fn walk_ast<'b>(&'b self, mut out: AstPass<'_, 'b, DB>) -> QueryResult<()> {
        out.push_sql(" RETURNING ");
        self.0.walk_ast(out.reborrow())?;
        Ok(())
    }
}

pub trait ReturningClauseHelper<S> {
    type WithReturning;
}

impl<S, T, U, Op> ReturningClauseHelper<S> for InsertStatement<T, U, Op>
where
    T: QuerySource,
{
    type WithReturning = InsertStatement<T, U, Op, ReturningClause<S>>;
}

impl<S, T, U, V> ReturningClauseHelper<S> for UpdateStatement<T, U, V>
where
    T: QuerySource,
{
    type WithReturning = UpdateStatement<T, U, V, ReturningClause<S>>;
}

impl<S, T, U> ReturningClauseHelper<S> for DeleteStatement<T, U>
where
    T: QuerySource,
{
    type WithReturning = DeleteStatement<T, U, ReturningClause<S>>;
}
