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

pub(crate) mod boxed;
mod dsl_impls;
#[diesel_derives::__diesel_public_if(
    feature = "i-implement-a-third-party-backend-and-opt-into-breaking-changes"
)]
pub(crate) use self::boxed::BoxedSelectStatement;

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
use crate::backend::{sql_dialect, Backend};
use crate::expression::subselect::ValidSubselect;
use crate::expression::*;
use crate::query_builder::having_clause::NoHavingClause;
use crate::query_builder::limit_offset_clause::LimitOffsetClause;
use crate::query_builder::{QueryId, SelectQuery};
use crate::query_dsl::order_dsl::ValidOrderingForDistinct;
use crate::query_source::joins::{AppendSelection, Inner, Join};
use crate::query_source::*;
use crate::result::QueryResult;

/// This type represents a select query
///
/// Using this type directly is only meaningful for custom backends
/// that need to provide a custom [`QueryFragment`] implementation
#[diesel_derives::__diesel_public_if(
    feature = "i-implement-a-third-party-backend-and-opt-into-breaking-changes",
    public_fields(
        select,
        from,
        distinct,
        where_clause,
        order,
        limit_offset,
        group_by,
        having,
        locking
    )
)]
#[derive(Debug, Clone, Copy, QueryId)]
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
    /// The select clause of the query
    pub(crate) select: Select,
    /// The from clause of the query
    pub(crate) from: From,
    /// The distinct clause of the query
    pub(crate) distinct: Distinct,
    /// The where clause of the query
    pub(crate) where_clause: Where,
    /// The order clause of the query
    pub(crate) order: Order,
    /// The combined limit/offset clause of the query
    pub(crate) limit_offset: LimitOffset,
    /// The group by clause of the query
    pub(crate) group_by: GroupBy,
    /// The having clause of the query
    pub(crate) having: Having,
    /// The locking clause of the query
    pub(crate) locking: Locking,
}

/// Semi-Private trait for containing get-functions for all `SelectStatement` fields
//
// This is used by `#[derive(MultiConnection)]`
pub trait SelectStatementAccessor {
    /// The type of the select clause
    type Select;
    /// The type of the from clause
    type From;
    /// The type of the distinct clause
    type Distinct;
    /// The type of the where clause
    type Where;
    /// The type of the order clause
    type Order;
    /// The type of the limit offset clause
    type LimitOffset;
    /// The type of the group by clause
    type GroupBy;
    /// The type of the having clause
    type Having;
    /// The type of the locking clause
    type Locking;

    /// Access the select clause
    fn select_clause(&self) -> &Self::Select;
    /// Access the from clause
    #[allow(clippy::wrong_self_convention)] // obviously wrong, as `from` refers to the clause name
    fn from_clause(&self) -> &Self::From;
    /// Access the distinct clause
    fn distinct_clause(&self) -> &Self::Distinct;
    /// Access the where clause
    fn where_clause(&self) -> &Self::Where;
    /// Access the order clause
    fn order_clause(&self) -> &Self::Order;
    /// Access the limit_offset clause
    fn limit_offset_clause(&self) -> &Self::LimitOffset;
    /// Access the group by clause
    fn group_by_clause(&self) -> &Self::GroupBy;
    /// Access the having clause
    fn having_clause(&self) -> &Self::Having;
    /// Access the locking clause
    fn locking_clause(&self) -> &Self::Locking;
}

impl<F, S, D, W, O, LOf, G, H, LC> SelectStatementAccessor
    for SelectStatement<F, S, D, W, O, LOf, G, H, LC>
{
    type Select = S;
    type From = F;
    type Distinct = D;
    type Where = W;
    type Order = O;
    type LimitOffset = LOf;
    type GroupBy = G;
    type Having = H;
    type Locking = LC;

    fn select_clause(&self) -> &Self::Select {
        &self.select
    }

    fn from_clause(&self) -> &Self::From {
        &self.from
    }

    fn distinct_clause(&self) -> &Self::Distinct {
        &self.distinct
    }

    fn where_clause(&self) -> &Self::Where {
        &self.where_clause
    }

    fn order_clause(&self) -> &Self::Order {
        &self.order
    }

    fn limit_offset_clause(&self) -> &Self::LimitOffset {
        &self.limit_offset
    }

    fn group_by_clause(&self) -> &Self::GroupBy {
        &self.group_by
    }

    fn having_clause(&self) -> &Self::Having {
        &self.having
    }

    fn locking_clause(&self) -> &Self::Locking {
        &self.locking
    }
}

impl<F, S, D, W, O, LOf, G, H, LC> SelectStatement<F, S, D, W, O, LOf, G, H, LC> {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new(
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
    // This is used by the `table!` macro
    #[doc(hidden)]
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
    Self: QueryFragment<DB, DB::SelectStatementSyntax>,
{
    fn walk_ast<'b>(&'b self, pass: AstPass<'_, 'b, DB>) -> QueryResult<()> {
        <Self as QueryFragment<DB, DB::SelectStatementSyntax>>::walk_ast(self, pass)
    }
}

impl<F, S, D, W, O, LOf, G, H, LC, DB>
    QueryFragment<DB, sql_dialect::select_statement_syntax::AnsiSqlSelectStatement>
    for SelectStatement<F, S, D, W, O, LOf, G, H, LC>
where
    DB: Backend<
        SelectStatementSyntax = sql_dialect::select_statement_syntax::AnsiSqlSelectStatement,
    >,
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
    fn walk_ast<'b>(&'b self, mut out: AstPass<'_, 'b, DB>) -> QueryResult<()> {
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
