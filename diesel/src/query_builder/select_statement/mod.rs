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
//! H: Having clause
#![allow(missing_docs)] // The missing_docs lint triggers even though this is hidden

mod boxed;
mod dsl_impls;

pub use self::boxed::BoxedSelectStatement;

use super::distinct_clause::NoDistinctClause;
use super::group_by_clause::*;
use super::having_clause::*;
use super::limit_clause::NoLimitClause;
use super::locking_clause::NoLockingClause;
use super::offset_clause::NoOffsetClause;
use super::order_clause::NoOrderClause;
use super::select_clause::*;
use super::where_clause::*;
use super::{AstPass, Query, QueryFragment};
use crate::backend::Backend;
use crate::expression::subselect::ValidSubselect;
use crate::expression::*;
use crate::query_builder::limit_offset_clause::LimitOffsetClause;
use crate::query_builder::{QueryId, SelectQuery};
use crate::query_source::joins::{AppendSelection, Inner, Join};
use crate::query_source::*;
use crate::result::QueryResult;

#[derive(Debug, Clone, Copy, QueryId)]
#[doc(hidden)]
#[must_use = "Queries are only executed when calling `load`, `get_result` or similar."]
pub struct SelectStatement<
    From,
    Select = DefaultSelectClause,
    Distinct = NoDistinctClause,
    Where = NoWhereClause,
    Order = NoOrderClause,
    LimitOffset = LimitOffsetClause<NoLimitClause, NoOffsetClause>,
    GroupBy = NoGroupByClause,
    Locking = NoLockingClause,
    Having = NoHavingClause,
> {
    pub(crate) select: Select,
    pub(crate) from: From,
    pub(crate) distinct: Distinct,
    pub(crate) where_clause: Where,
    pub(crate) order: Order,
    pub(crate) limit_offset: LimitOffset,
    pub(crate) group_by: GroupBy,
    pub(crate) locking: Locking,
    pub(crate) having: Having,
}

impl<F, S, D, W, O, LOf, G, LC, H> SelectStatement<F, S, D, W, O, LOf, G, LC, H> {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        select: S,
        from: F,
        distinct: D,
        where_clause: W,
        order: O,
        limit_offset: LOf,
        group_by: G,
        locking: LC,
        having: H,
    ) -> Self {
        SelectStatement {
            select,
            from,
            distinct,
            where_clause,
            order,
            limit_offset,
            group_by,
            locking,
            having,
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
            LimitOffsetClause {
                limit_clause: NoLimitClause,
                offset_clause: NoOffsetClause,
            },
            NoGroupByClause,
            NoLockingClause,
            NoHavingClause,
        )
    }
}

impl<F, S, D, W, O, LOf, G, LC, H> Query for SelectStatement<F, S, D, W, O, LOf, G, LC, H>
where
    G: ValidGroupByClause,
    S: SelectClauseExpression<F>,
    S::Selection: ValidGrouping<G::Expressions>,
    W: ValidWhereClause<F>,
{
    type SqlType = S::SelectClauseSqlType;
}

impl<F, S, D, W, O, LOf, G, LC, H> SelectQuery for SelectStatement<F, S, D, W, O, LOf, G, LC, H>
where
    S: SelectClauseExpression<F>,
{
    type SqlType = S::SelectClauseSqlType;
}

impl<F, S, D, W, O, LOf, G, LC, DB, H> QueryFragment<DB>
    for SelectStatement<F, S, D, W, O, LOf, G, LC, H>
where
    DB: Backend,
    S: SelectClauseQueryFragment<F, DB>,
    F: QuerySource,
    F::FromClause: QueryFragment<DB>,
    D: QueryFragment<DB>,
    W: QueryFragment<DB>,
    O: QueryFragment<DB>,
    LOf: QueryFragment<DB>,
    G: QueryFragment<DB>,
    LC: QueryFragment<DB>,
    H: QueryFragment<DB>,
{
    fn walk_ast(&self, mut out: AstPass<DB>) -> QueryResult<()> {
        out.push_sql("SELECT ");
        self.distinct.walk_ast(out.reborrow())?;
        self.select.walk_ast(&self.from, out.reborrow())?;
        out.push_sql(" FROM ");
        self.from.from_clause().walk_ast(out.reborrow())?;
        self.where_clause.walk_ast(out.reborrow())?;
        self.group_by.walk_ast(out.reborrow())?;
        self.having.walk_ast(out.reborrow())?;
        self.order.walk_ast(out.reborrow())?;
        self.limit_offset.walk_ast(out.reborrow())?;
        self.locking.walk_ast(out.reborrow())?;
        Ok(())
    }
}

impl<S, D, W, O, LOf, G, LC, H, DB> QueryFragment<DB>
    for SelectStatement<(), S, D, W, O, LOf, G, LC, H>
where
    DB: Backend,
    S: SelectClauseQueryFragment<(), DB>,
    D: QueryFragment<DB>,
    W: QueryFragment<DB>,
    O: QueryFragment<DB>,
    LOf: QueryFragment<DB>,
    G: QueryFragment<DB>,
    LC: QueryFragment<DB>,
    H: QueryFragment<DB>,
{
    fn walk_ast(&self, mut out: AstPass<DB>) -> QueryResult<()> {
        out.push_sql("SELECT ");
        self.distinct.walk_ast(out.reborrow())?;
        self.select.walk_ast(&(), out.reborrow())?;
        self.where_clause.walk_ast(out.reborrow())?;
        self.group_by.walk_ast(out.reborrow())?;
        self.having.walk_ast(out.reborrow())?;
        self.order.walk_ast(out.reborrow())?;
        self.limit_offset.walk_ast(out.reborrow())?;
        self.locking.walk_ast(out.reborrow())?;
        Ok(())
    }
}

impl<S, F, D, W, O, LOf, G, LC, H, QS> ValidSubselect<QS>
    for SelectStatement<F, S, D, W, O, LOf, LC, G, H>
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
