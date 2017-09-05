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
mod dsl_impls;
mod boxed;

pub use self::boxed::BoxedSelectStatement;

use backend::Backend;
use expression::*;
use query_source::*;
use query_source::joins::AppendSelection;
use result::QueryResult;
use super::distinct_clause::NoDistinctClause;
use super::group_by_clause::NoGroupByClause;
use super::limit_clause::NoLimitClause;
use super::offset_clause::NoOffsetClause;
use super::order_clause::NoOrderClause;
use super::select_clause::*;
use super::where_clause::NoWhereClause;
use super::{AstPass, Query, QueryFragment};

#[derive(Debug, Clone, Copy)]
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
> {
    select: Select,
    from: From,
    distinct: Distinct,
    where_clause: Where,
    order: Order,
    limit: Limit,
    offset: Offset,
    group_by: GroupBy,
}

impl<F, S, D, W, O, L, Of, G> SelectStatement<F, S, D, W, O, L, Of, G> {
    #[cfg_attr(feature = "clippy", allow(too_many_arguments))]
    pub fn new(
        select: S,
        from: F,
        distinct: D,
        where_clause: W,
        order: O,
        limit: L,
        offset: Of,
        group_by: G,
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
        )
    }
}

impl<F, S, D, W, O, L, Of, G> Query for SelectStatement<F, S, D, W, O, L, Of, G>
where
    S: SelectClauseExpression<F>,
{
    type SqlType = S::SelectClauseSqlType;
}

#[cfg(feature = "postgres")]
impl<F, S, D, W, O, L, Of, G> Expression for SelectStatement<F, S, D, W, O, L, Of, G>
where
    S: SelectClauseExpression<F>,
{
    type SqlType = ::types::Array<S::SelectClauseSqlType>;
}

#[cfg(not(feature = "postgres"))]
impl<F, S, D, W, O, L, Of, G> Expression for SelectStatement<F, S, D, W, O, L, Of, G>
where
    S: SelectClauseExpression<F>,
{
    type SqlType = S::SelectClauseSqlType;
}

impl<F, S, D, W, O, L, Of, G, DB> QueryFragment<DB> for SelectStatement<F, S, D, W, O, L, Of, G>
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
        Ok(())
    }
}

impl<S, D, W, O, L, Of, G, DB> QueryFragment<DB> for SelectStatement<(), S, D, W, O, L, Of, G>
where
    DB: Backend,
    S: SelectClauseQueryFragment<(), DB>,
    D: QueryFragment<DB>,
    W: QueryFragment<DB>,
    O: QueryFragment<DB>,
    L: QueryFragment<DB>,
    Of: QueryFragment<DB>,
    G: QueryFragment<DB>,
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
        Ok(())
    }
}

impl_query_id!(SelectStatement<F, S, D, W, O, L, Of, G>);

impl<F, S, D, W, O, L, Of, G, QS> SelectableExpression<QS>
    for SelectStatement<F, S, D, W, O, L, Of, G>
where
    SelectStatement<F, S, D, W, O, L, Of, G>: AppearsOnTable<QS>,
{
}

impl<S, F, D, W, O, L, Of, G, QS> AppearsOnTable<QS> for SelectStatement<S, F, D, W, O, L, Of, G>
where
    SelectStatement<S, F, D, W, O, L, Of, G>: Expression,
{
}

impl<F, S, D, W, O, L, Of, G> NonAggregate for SelectStatement<F, S, D, W, O, L, Of, G>
where
    SelectStatement<F, S, D, W, O, L, Of, G>: Expression,
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
