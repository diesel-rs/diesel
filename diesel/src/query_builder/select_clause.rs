use super::from_clause::AsQuerySource;
use crate::expression::{Expression, SelectableExpression};
use crate::query_builder::*;
use crate::query_source::QuerySource;

#[doc(hidden)]
pub struct DefaultSelectClause<QS: AsQuerySource> {
    default_selection: <QS::QuerySource as QuerySource>::DefaultSelection,
}

impl<QS> std::fmt::Debug for DefaultSelectClause<QS>
where
    QS: AsQuerySource,
    <QS::QuerySource as QuerySource>::DefaultSelection: std::fmt::Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DefaultSelectClause")
            .field("default_selection", &self.default_selection)
            .finish()
    }
}

impl<QS> Clone for DefaultSelectClause<QS>
where
    QS: AsQuerySource,
    <QS::QuerySource as QuerySource>::DefaultSelection: Clone,
{
    fn clone(&self) -> Self {
        Self {
            default_selection: self.default_selection.clone(),
        }
    }
}

impl<QS> Copy for DefaultSelectClause<QS>
where
    QS: AsQuerySource,
    <QS::QuerySource as QuerySource>::DefaultSelection: Copy,
{
}

impl<QS: AsQuerySource> DefaultSelectClause<QS> {
    pub(crate) fn new(qs: &QS) -> Self {
        Self {
            default_selection: qs.as_query_source().default_selection(),
        }
    }
}

impl<QS> QueryId for DefaultSelectClause<QS>
where
    QS: AsQuerySource,
    <QS::QuerySource as QuerySource>::DefaultSelection: QueryId,
{
    type QueryId = <<QS::QuerySource as QuerySource>::DefaultSelection as QueryId>::QueryId;

    const HAS_STATIC_QUERY_ID: bool =
        <<QS::QuerySource as QuerySource>::DefaultSelection as QueryId>::HAS_STATIC_QUERY_ID;
}

#[derive(Debug, Clone, Copy, QueryId)]
pub struct SelectClause<T>(pub T);

/// Specialised variant of `Expression` for select clause types
///
/// The difference to the normal `Expression` trait is the query source (`QS`)
/// generic type parameter. This allows to access the query source in generic code.
#[cfg_attr(
    feature = "i-implement-a-third-party-backend-and-opt-into-breaking-changes",
    cfg(feature = "i-implement-a-third-party-backend-and-opt-into-breaking-changes")
)]
pub trait SelectClauseExpression<QS> {
    /// The expression represented by the given select clause
    type Selection;
    /// SQL type of the select clause
    type SelectClauseSqlType;
}

impl<T, QS> SelectClauseExpression<FromClause<QS>> for SelectClause<T>
where
    QS: QuerySource,
    T: SelectableExpression<QS>,
{
    type Selection = T;
    type SelectClauseSqlType = T::SqlType;
}

impl<T> SelectClauseExpression<NoFromClause> for SelectClause<T>
where
    T: SelectableExpression<NoFromClause>,
{
    type Selection = T;
    type SelectClauseSqlType = T::SqlType;
}

impl<QS> SelectClauseExpression<FromClause<QS>> for DefaultSelectClause<FromClause<QS>>
where
    QS: QuerySource,
{
    type Selection = QS::DefaultSelection;
    type SelectClauseSqlType = <Self::Selection as Expression>::SqlType;
}

impl<T, DB> QueryFragment<DB> for SelectClause<T>
where
    DB: Backend,
    T: QueryFragment<DB>,
{
    fn walk_ast<'b>(&'b self, pass: AstPass<'_, 'b, DB>) -> QueryResult<()> {
        self.0.walk_ast(pass)
    }
}

impl<QS, DB> QueryFragment<DB> for DefaultSelectClause<QS>
where
    DB: Backend,
    QS: AsQuerySource,
    <QS::QuerySource as QuerySource>::DefaultSelection: QueryFragment<DB>,
{
    fn walk_ast<'b>(&'b self, pass: AstPass<'_, 'b, DB>) -> QueryResult<()> {
        self.default_selection.walk_ast(pass)
    }
}
