use super::{AstPass, QueryFragment, QueryId};
use crate::backend::Backend;
use crate::query_source::AppearsInFromClause;
use crate::{QueryResult, QuerySource};

/// This type represents a not existing from clause
///
/// Custom backends can provide a custom [`QueryFragment`]
/// impl by specializing the implementation via
/// [`SqlDialect::EmptyFromClauseSyntax`](crate::backend::SqlDialect::EmptyFromClauseSyntax)
#[cfg_attr(
    feature = "i-implement-a-third-party-backend-and-opt-into-breaking-changes",
    cfg(feature = "i-implement-a-third-party-backend-and-opt-into-breaking-changes")
)]
#[derive(Debug, Clone, Copy, QueryId)]
pub struct NoFromClause;

impl<DB> QueryFragment<DB> for NoFromClause
where
    Self: QueryFragment<DB, DB::EmptyFromClauseSyntax>,
    DB: Backend,
{
    fn walk_ast<'b>(&'b self, pass: AstPass<'_, 'b, DB>) -> QueryResult<()> {
        <Self as QueryFragment<DB, DB::EmptyFromClauseSyntax>>::walk_ast(self, pass)
    }
}

impl<DB> QueryFragment<DB, crate::backend::sql_dialect::from_clause_syntax::AnsiSqlFromClauseSyntax>
    for NoFromClause
where
    DB: Backend<EmptyFromClauseSyntax = crate::backend::sql_dialect::from_clause_syntax::AnsiSqlFromClauseSyntax>,
{
    fn walk_ast<'b>(&'b self, _pass: AstPass<'_, 'b, DB>) -> QueryResult<()> {
        Ok(())
    }
}

pub trait AsQuerySource {
    type QuerySource: QuerySource;

    fn as_query_source(&self) -> &Self::QuerySource;
}

impl<QS> AsQuerySource for QS
where
    QS: QuerySource,
{
    type QuerySource = Self;

    fn as_query_source(&self) -> &Self::QuerySource {
        self
    }
}

#[doc(hidden)]
pub struct FromClause<F: QuerySource> {
    pub(crate) source: F,
    pub(crate) from_clause: F::FromClause,
}

impl<F> AsQuerySource for FromClause<F>
where
    F: QuerySource,
{
    type QuerySource = F;

    fn as_query_source(&self) -> &Self::QuerySource {
        &self.source
    }
}

impl<F> QueryId for FromClause<F>
where
    F: QuerySource + QueryId,
{
    type QueryId = F::QueryId;

    const HAS_STATIC_QUERY_ID: bool = F::HAS_STATIC_QUERY_ID;
}

impl<F> Copy for FromClause<F>
where
    F: QuerySource + Copy,
    F::FromClause: Copy,
{
}

impl<F> Clone for FromClause<F>
where
    F: QuerySource + Clone,
    F::FromClause: Clone,
{
    fn clone(&self) -> Self {
        Self {
            source: self.source.clone(),
            from_clause: self.from_clause.clone(),
        }
    }
}

impl<F> std::fmt::Debug for FromClause<F>
where
    F: QuerySource,
    F::FromClause: std::fmt::Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FromClause")
            .field("from_clause", &self.from_clause)
            .finish()
    }
}

impl<F: QuerySource> FromClause<F> {
    pub(crate) fn new(qs: F) -> Self {
        Self {
            from_clause: qs.from_clause(),
            source: qs,
        }
    }
}

impl<DB, F> QueryFragment<DB> for FromClause<F>
where
    F: QuerySource,
    DB: Backend,
    F::FromClause: QueryFragment<DB>,
{
    fn walk_ast<'b>(&'b self, mut pass: AstPass<'_, 'b, DB>) -> QueryResult<()> {
        pass.push_sql(" FROM ");
        self.from_clause.walk_ast(pass)
    }
}

impl<QS1, QS2> AppearsInFromClause<QS1> for FromClause<QS2>
where
    QS1: QuerySource,
    QS2: QuerySource,
    QS2: AppearsInFromClause<QS1>,
{
    type Count = <QS2 as AppearsInFromClause<QS1>>::Count;
}
