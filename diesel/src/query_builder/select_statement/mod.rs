//! Within this module, types commonly use the following abbreviations:
//!
//! F: From Clause
//! S: Select Clause
//! D: Distinct Clause
//! W: Where Clause
//! O: Order By Clause
//! L: Limit Clause
//! Of: Offset Clause
//! G: Group By Clause
//! LC: For Update Clause
#![allow(missing_docs)] // The missing_docs lint triggers even though this is hidden

mod boxed;
mod dsl_impls;

pub use self::boxed::BoxedSelectStatement;

use super::distinct_clause::NoDistinctClause;
use super::group_by_clause::NoGroupByClause;
use super::limit_clause::NoLimitClause;
use super::locking_clause::NoLockingClause;
use super::offset_clause::NoOffsetClause;
use super::order_clause::NoOrderClause;
use super::select_clause::*;
use super::where_clause::*;
use super::{AstPass, Query, QueryFragment};
use backend::Backend;
use expression::subselect::ValidSubselect;
use expression::*;
use query_builder::SelectQuery;
use query_source::joins::{AppendSelection, Inner, Join};
use query_source::*;
use result::QueryResult;

#[derive(Debug, Clone, Copy, QueryId)]
#[doc(hidden)]
#[must_use = "Queries are only executed when calling `load`, `get_result` or similar."]
pub struct SelectStatement<
    From,
    Select = DefaultSelectClause,
    Distinct = NoDistinctClause,
    Where = NoWhereClause,
    Order = NoOrderClause,
    Limit = NoLimitClause,
    Offset = NoOffsetClause,
    GroupBy = NoGroupByClause,
    Locking = NoLockingClause,
> {
    pub(crate) select: Select,
    pub(crate) from: From,
    pub(crate) distinct: Distinct,
    pub(crate) where_clause: Where,
    pub(crate) order: Order,
    pub(crate) limit: Limit,
    pub(crate) offset: Offset,
    pub(crate) group_by: GroupBy,
    pub(crate) locking: Locking,
}

impl<F, S, D, W, O, L, Of, G, LC> SelectStatement<F, S, D, W, O, L, Of, G, LC> {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        select: S,
        from: F,
        distinct: D,
        where_clause: W,
        order: O,
        limit: L,
        offset: Of,
        group_by: G,
        locking: LC,
    ) -> Self {
        SelectStatement {
            select: select,
            from: from,
            distinct: distinct,
            where_clause: where_clause,
            order: order,
            limit: limit,
            offset: offset,
            group_by: group_by,
            locking: locking,
        }
    }
}

impl<F> SelectStatement<F> {
    pub fn simple(from: F) -> Self {
        SelectStatement::new(
            DefaultSelectClause,
            from,
            NoDistinctClause,
            NoWhereClause,
            NoOrderClause,
            NoLimitClause,
            NoOffsetClause,
            NoGroupByClause,
            NoLockingClause,
        )
    }
}

impl<F, S, D, W, O, L, Of, G, LC> Query for SelectStatement<F, S, D, W, O, L, Of, G, LC>
where
    S: SelectClauseExpression<F>,
    W: ValidWhereClause<F>,
{
    type SqlType = S::SelectClauseSqlType;
}

impl<F, S, D, W, O, L, Of, G, LC> SelectQuery for SelectStatement<F, S, D, W, O, L, Of, G, LC>
where
    S: SelectClauseExpression<F>,
{
    type SqlType = S::SelectClauseSqlType;
}

impl<F, S, D, W, O, L, Of, G, LC, DB> QueryFragment<DB>
    for SelectStatement<F, S, D, W, O, L, Of, G, LC>
where
    DB: Backend,
    S: SelectClauseQueryFragment<F, DB>,
    F: QuerySource,
    F::FromClause: QueryFragment<DB>,
    D: QueryFragment<DB>,
    W: QueryFragment<DB>,
    O: QueryFragment<DB>,
    L: QueryFragment<DB>,
    Of: QueryFragment<DB>,
    G: QueryFragment<DB>,
    LC: QueryFragment<DB>,
{
    fn walk_ast(&self, mut out: AstPass<DB>) -> QueryResult<()> {
        out.push_sql("SELECT ");
        self.distinct.walk_ast(out.reborrow())?;
        self.select.walk_ast(&self.from, out.reborrow())?;
        out.push_sql(" FROM ");
        self.from.from_clause().walk_ast(out.reborrow())?;
        self.where_clause.walk_ast(out.reborrow())?;
        self.group_by.walk_ast(out.reborrow())?;
        self.order.walk_ast(out.reborrow())?;
        self.limit.walk_ast(out.reborrow())?;
        self.offset.walk_ast(out.reborrow())?;
        self.locking.walk_ast(out.reborrow())?;
        Ok(())
    }
}

impl<S, D, W, O, L, Of, G, LC, DB> QueryFragment<DB>
    for SelectStatement<(), S, D, W, O, L, Of, G, LC>
where
    DB: Backend,
    S: SelectClauseQueryFragment<(), DB>,
    D: QueryFragment<DB>,
    W: QueryFragment<DB>,
    O: QueryFragment<DB>,
    L: QueryFragment<DB>,
    Of: QueryFragment<DB>,
    G: QueryFragment<DB>,
    LC: QueryFragment<DB>,
{
    fn walk_ast(&self, mut out: AstPass<DB>) -> QueryResult<()> {
        out.push_sql("SELECT ");
        self.distinct.walk_ast(out.reborrow())?;
        self.select.walk_ast(&(), out.reborrow())?;
        self.where_clause.walk_ast(out.reborrow())?;
        self.group_by.walk_ast(out.reborrow())?;
        self.order.walk_ast(out.reborrow())?;
        self.limit.walk_ast(out.reborrow())?;
        self.offset.walk_ast(out.reborrow())?;
        self.locking.walk_ast(out.reborrow())?;
        Ok(())
    }
}

impl<S, F, D, W, O, L, Of, G, LC, QS> ValidSubselect<QS>
    for SelectStatement<F, S, D, W, O, L, Of, LC, G>
where
    Self: SelectQuery,
    W: ValidWhereClause<Join<F, QS, Inner>>,
{
}

/// Allow `SelectStatement<From>` to act as if it were `From` as long as
/// no other query methods have been called on it
impl<From, T> AppearsInFromClause<T> for SelectStatement<From>
where
    From: AppearsInFromClause<T>,
{
    type Count = From::Count;
}

impl<From> QuerySource for SelectStatement<From>
where
    From: QuerySource,
    From::DefaultSelection: SelectableExpression<Self>,
{
    type FromClause = From::FromClause;
    type DefaultSelection = From::DefaultSelection;

    fn from_clause(&self) -> Self::FromClause {
        self.from.from_clause()
    }

    fn default_selection(&self) -> Self::DefaultSelection {
        self.from.default_selection()
    }
}

impl<From, Selection> AppendSelection<Selection> for SelectStatement<From>
where
    From: AppendSelection<Selection>,
{
    type Output = From::Output;

    fn append_selection(&self, selection: Selection) -> Self::Output {
        self.from.append_selection(selection)
    }
}
