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
//! H: Having clause
//! LC: For Update Clause
#![allow(missing_docs)] // The missing_docs lint triggers even though this is hidden

mod boxed;
mod dsl_impls;

pub use self::boxed::BoxedSelectStatement;

use super::distinct_clause::NoDistinctClause;
use super::from_clause::AsQuerySource;
use super::from_clause::FromClause;
use super::group_by_clause::*;
use super::limit_clause::NoLimitClause;
use super::locking_clause::NoLockingClause;
use super::offset_clause::NoOffsetClause;
use super::order_clause::NoOrderClause;
use super::select_clause::*;
use super::where_clause::*;
use super::NoFromClause;
use super::{AstPass, Query, QueryFragment};
use crate::backend::Backend;
use crate::expression::subselect::ValidSubselect;
use crate::expression::*;
use crate::query_builder::having_clause::NoHavingClause;
use crate::query_builder::limit_offset_clause::LimitOffsetClause;
use crate::query_builder::{QueryId, SelectQuery};
use crate::query_dsl::order_dsl::ValidOrderingForDistinct;
use crate::query_source::joins::{AppendSelection, Inner, Join};
use crate::query_source::*;
use crate::result::QueryResult;

#[derive(Debug, Clone, Copy, QueryId)]
#[doc(hidden)]
#[must_use = "Queries are only executed when calling `load`, `get_result` or similar."]
pub struct SelectStatement<
    From,
    Select = DefaultSelectClause<From>,
    Distinct = NoDistinctClause,
    Where = NoWhereClause,
    Order = NoOrderClause,
    LimitOffset = LimitOffsetClause<NoLimitClause, NoOffsetClause>,
    GroupBy = NoGroupByClause,
    Having = NoHavingClause,
    Locking = NoLockingClause,
> {
    pub(crate) select: Select,
    pub(crate) from: From,
    pub(crate) distinct: Distinct,
    pub(crate) where_clause: Where,
    pub(crate) order: Order,
    pub(crate) limit_offset: LimitOffset,
    pub(crate) group_by: GroupBy,
    pub(crate) having: Having,
    pub(crate) locking: Locking,
}

impl<F, S, D, W, O, LOf, G, H, LC> SelectStatement<F, S, D, W, O, LOf, G, H, LC> {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        select: S,
        from: F,
        distinct: D,
        where_clause: W,
        order: O,
        limit_offset: LOf,
        group_by: G,
        having: H,
        locking: LC,
    ) -> Self {
        SelectStatement {
            select,
            from,
            distinct,
            where_clause,
            order,
            limit_offset,
            group_by,
            having,
            locking,
        }
    }
}

impl<F: QuerySource> SelectStatement<FromClause<F>> {
    pub fn simple(from: F) -> Self {
        let from = FromClause::new(from);
        SelectStatement::new(
            DefaultSelectClause::new(&from),
            from,
            NoDistinctClause,
            NoWhereClause,
            NoOrderClause,
            LimitOffsetClause {
                limit_clause: NoLimitClause,
                offset_clause: NoOffsetClause,
            },
            NoGroupByClause,
            NoHavingClause,
            NoLockingClause,
        )
    }
}

impl<F, S, D, W, O, LOf, G, H, LC> Query for SelectStatement<F, S, D, W, O, LOf, G, H, LC>
where
    G: ValidGroupByClause,
    S: SelectClauseExpression<F>,
    S::Selection: ValidGrouping<G::Expressions>,
    W: ValidWhereClause<F>,
{
    type SqlType = S::SelectClauseSqlType;
}

impl<F, S, D, W, O, LOf, G, H, LC> SelectQuery for SelectStatement<F, S, D, W, O, LOf, G, H, LC>
where
    S: SelectClauseExpression<F>,
    O: ValidOrderingForDistinct<D>,
{
    type SqlType = S::SelectClauseSqlType;
}

impl<F, S, D, W, O, LOf, G, H, LC, DB> QueryFragment<DB>
    for SelectStatement<F, S, D, W, O, LOf, G, H, LC>
where
    DB: Backend,
    S: QueryFragment<DB>,
    F: QueryFragment<DB>,
    D: QueryFragment<DB>,
    W: QueryFragment<DB>,
    O: QueryFragment<DB>,
    LOf: QueryFragment<DB>,
    G: QueryFragment<DB>,
    H: QueryFragment<DB>,
    LC: QueryFragment<DB>,
{
    fn walk_ast<'a, 'b>(&'a self, mut out: AstPass<'_, 'b, DB>) -> QueryResult<()>
    where
        'a: 'b,
    {
        out.push_sql("SELECT ");
        self.distinct.walk_ast(out.reborrow())?;
        self.select.walk_ast(out.reborrow())?;
        self.from.walk_ast(out.reborrow())?;
        self.where_clause.walk_ast(out.reborrow())?;
        self.group_by.walk_ast(out.reborrow())?;
        self.having.walk_ast(out.reborrow())?;
        self.order.walk_ast(out.reborrow())?;
        self.limit_offset.walk_ast(out.reborrow())?;
        self.locking.walk_ast(out.reborrow())?;
        Ok(())
    }
}

impl<S, F, D, W, O, LOf, G, H, LC, QS> ValidSubselect<QS>
    for SelectStatement<FromClause<F>, S, D, W, O, LOf, G, H, LC>
where
    Self: SelectQuery,
    F: QuerySource,
    QS: QuerySource,
    Join<F, QS, Inner>: QuerySource,
    W: ValidWhereClause<FromClause<Join<F, QS, Inner>>>,
{
}

impl<S, D, W, O, LOf, G, H, LC> ValidSubselect<NoFromClause>
    for SelectStatement<NoFromClause, S, D, W, O, LOf, G, H, LC>
where
    Self: SelectQuery,
    W: ValidWhereClause<NoFromClause>,
{
}

impl<S, F, D, W, O, LOf, G, H, LC> ValidSubselect<NoFromClause>
    for SelectStatement<FromClause<F>, S, D, W, O, LOf, G, H, LC>
where
    Self: SelectQuery,
    F: QuerySource,
    W: ValidWhereClause<FromClause<F>>,
{
}

impl<S, D, W, O, LOf, G, H, LC, QS> ValidSubselect<QS>
    for SelectStatement<NoFromClause, S, D, W, O, LOf, G, H, LC>
where
    Self: SelectQuery,
    QS: QuerySource,
    W: ValidWhereClause<NoFromClause>,
{
}

/// Allow `SelectStatement<From>` to act as if it were `From` as long as
/// no other query methods have been called on it
impl<From, T> AppearsInFromClause<T> for SelectStatement<From>
where
    From: AsQuerySource,
    From::QuerySource: AppearsInFromClause<T> + QuerySource,
{
    type Count = <From::QuerySource as AppearsInFromClause<T>>::Count;
}

impl<From> QuerySource for SelectStatement<From>
where
    From: AsQuerySource,
    <From::QuerySource as QuerySource>::DefaultSelection: SelectableExpression<Self>,
{
    type FromClause = <From::QuerySource as QuerySource>::FromClause;
    type DefaultSelection = <From::QuerySource as QuerySource>::DefaultSelection;

    fn from_clause(&self) -> <From::QuerySource as QuerySource>::FromClause {
        self.from.as_query_source().from_clause()
    }

    fn default_selection(&self) -> Self::DefaultSelection {
        self.from.as_query_source().default_selection()
    }
}

impl<From, Selection> AppendSelection<Selection> for SelectStatement<From>
where
    From: AsQuerySource,
    From::QuerySource: AppendSelection<Selection>,
{
    type Output = <From::QuerySource as AppendSelection<Selection>>::Output;

    fn append_selection(&self, selection: Selection) -> Self::Output {
        self.from.as_query_source().append_selection(selection)
    }
}
