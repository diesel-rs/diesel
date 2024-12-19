use crate::backend::sql_dialect;
use crate::query_builder::{QueryFragment, QueryId};
use crate::serialize::ToSql;
use crate::sql_types::BigInt;

use super::aggregate_order::NoOrder;
use super::over_clause::OverClause;
use super::prefix::NoPrefix;
use super::within_group::NoWithin;
use super::AggregateExpression;

empty_clause!(NoFrame);

#[derive(QueryId, Copy, Clone, Debug)]
pub struct FrameClause<F>(F);

impl<F, DB> QueryFragment<DB> for FrameClause<F>
where
    F: QueryFragment<DB>,
    DB: crate::backend::Backend,
{
    fn walk_ast<'b>(
        &'b self,
        pass: crate::query_builder::AstPass<'_, 'b, DB>,
    ) -> crate::QueryResult<()> {
        self.0.walk_ast(pass)?;
        Ok(())
    }
}

macro_rules! simple_frame_expr {
    ($name: ident, $kind: expr) => {
        #[derive(QueryId, Clone, Copy, Debug)]
        #[doc(hidden)]
        pub struct $name;

        impl<DB> QueryFragment<DB> for $name
        where
            DB: crate::backend::Backend,
        {
            fn walk_ast<'b>(
                &'b self,
                mut pass: crate::query_builder::AstPass<'_, 'b, DB>,
            ) -> crate::QueryResult<()> {
                pass.push_sql($kind);
                Ok(())
            }
        }
    };
}

// kinds
simple_frame_expr!(Range, " RANGE ");
simple_frame_expr!(Rows, " ROWS ");

#[derive(QueryId, Clone, Copy, Debug)]
#[doc(hidden)]
pub struct Groups;

impl<DB> QueryFragment<DB> for Groups
where
    DB: crate::backend::Backend,
    Self: QueryFragment<DB, DB::WindowFrameClauseGroupSupport>,
{
    fn walk_ast<'b>(
        &'b self,
        pass: crate::query_builder::AstPass<'_, 'b, DB>,
    ) -> crate::QueryResult<()> {
        <Self as QueryFragment<DB, DB::WindowFrameClauseGroupSupport>>::walk_ast(self, pass)
    }
}
impl<DB> QueryFragment<DB, sql_dialect::window_frame_clause_group_support::IsoGroupWindowFrameUnit>
    for Groups
where
    DB: crate::backend::Backend<WindowFrameClauseGroupSupport = sql_dialect::window_frame_clause_group_support::IsoGroupWindowFrameUnit>,
{
    fn walk_ast<'b>(
        &'b self,
        mut pass: crate::query_builder::AstPass<'_, 'b, DB>,
    ) -> crate::QueryResult<()> {
        pass.push_sql(" GROUPS ");
        Ok(())
    }
}

// start & end
simple_frame_expr!(UnboundedPreceding, "UNBOUNDED PRECEDING ");
simple_frame_expr!(CurrentRow, "CURRENT ROW ");
simple_frame_expr!(UnboundedFollowing, "UNBOUNDED FOLLOWING ");

// exclusion
simple_frame_expr!(ExcludeCurrentRow, "EXCLUDE CURRENT ROW ");
simple_frame_expr!(ExcludeGroup, "EXCLUDE GROUP ");
simple_frame_expr!(ExcludeTies, "EXCLUDE TIES ");
simple_frame_expr!(ExcludeNoOthers, "EXCLUDE NO OTHERS ");

#[derive(QueryId, Clone, Copy, Debug)]
pub struct OffsetPreceding(i64);

impl<DB> QueryFragment<DB> for OffsetPreceding
where
    DB: crate::backend::Backend,
    i64: ToSql<BigInt, DB>,
{
    fn walk_ast<'b>(
        &'b self,
        mut pass: crate::query_builder::AstPass<'_, 'b, DB>,
    ) -> crate::QueryResult<()> {
        pass.push_bind_param::<BigInt, _>(&self.0)?;
        pass.push_sql(" PRECEDING ");
        Ok(())
    }
}

#[derive(QueryId, Clone, Copy, Debug)]
pub struct OffsetFollowing(i64);

impl<DB> QueryFragment<DB> for OffsetFollowing
where
    DB: crate::backend::Backend,
    i64: ToSql<BigInt, DB>,
{
    fn walk_ast<'b>(
        &'b self,
        mut pass: crate::query_builder::AstPass<'_, 'b, DB>,
    ) -> crate::QueryResult<()> {
        pass.push_bind_param::<BigInt, i64>(&self.0)?;
        pass.push_sql(" FOLLOWING ");
        Ok(())
    }
}

pub trait FrameDsl<F> {
    type Output;

    fn frame(self, expr: F) -> Self::Output;
}

impl<E, Fn, Filter, Frame, Partition, Order> FrameDsl<E>
    for AggregateExpression<
        Fn,
        NoPrefix,
        NoOrder,
        Filter,
        NoWithin,
        OverClause<Partition, Order, Frame>,
    >
where
    E: FrameClauseExpression,
{
    type Output = AggregateExpression<
        Fn,
        NoPrefix,
        NoOrder,
        Filter,
        NoWithin,
        OverClause<Partition, Order, FrameClause<E>>,
    >;

    fn frame(self, expr: E) -> Self::Output {
        AggregateExpression {
            prefix: self.prefix,
            function: self.function,
            order: self.order,
            filter: self.filter,
            within_group: self.within_group,
            window: OverClause {
                partition_by: self.window.partition_by,
                order: self.window.order,
                frame_clause: FrameClause(expr),
            },
        }
    }
}

pub trait FrameClauseExpression {}

pub trait FrameClauseStartBound {}
pub trait FrameClauseEndBound {}

impl FrameClauseEndBound for UnboundedFollowing {}
impl FrameClauseStartBound for UnboundedPreceding {}
impl FrameClauseEndBound for CurrentRow {}
impl FrameClauseStartBound for CurrentRow {}
impl FrameClauseStartBound for OffsetFollowing {}
impl FrameClauseEndBound for OffsetFollowing {}
impl FrameClauseStartBound for OffsetPreceding {}
impl FrameClauseEndBound for OffsetPreceding {}

pub trait FrameCauseExclusion {}

impl FrameCauseExclusion for ExcludeGroup {}
impl FrameCauseExclusion for ExcludeNoOthers {}
impl FrameCauseExclusion for ExcludeTies {}
impl FrameCauseExclusion for ExcludeCurrentRow {}

/// Construct a frame clause for window functions from an integer
pub trait FrameBoundDsl {
    /// Use the preceding frame clause specification
    fn preceding(self) -> OffsetPreceding;

    /// Use the following frame clause specification
    fn following(self) -> OffsetFollowing;
}

impl FrameBoundDsl for i64 {
    fn preceding(self) -> OffsetPreceding {
        OffsetPreceding(self)
    }

    fn following(self) -> OffsetFollowing {
        OffsetFollowing(self)
    }
}

empty_clause!(NoExclusion);

#[derive(QueryId, Copy, Clone, Debug)]
pub struct StartFrame<Kind, Start, Exclusion = NoExclusion> {
    kind: Kind,
    start: Start,
    exclusion: Exclusion,
}

impl<Kind, Start, Exclusion, DB> QueryFragment<DB> for StartFrame<Kind, Start, Exclusion>
where
    Kind: QueryFragment<DB>,
    Start: QueryFragment<DB>,
    Exclusion: QueryFragment<DB>,
    DB: crate::backend::Backend,
{
    fn walk_ast<'b>(
        &'b self,
        mut pass: crate::query_builder::AstPass<'_, 'b, DB>,
    ) -> crate::QueryResult<()> {
        self.kind.walk_ast(pass.reborrow())?;
        self.start.walk_ast(pass.reborrow())?;
        self.exclusion.walk_ast(pass.reborrow())?;
        Ok(())
    }
}

impl<Kind, Start, Exclusion> FrameClauseExpression for StartFrame<Kind, Start, Exclusion> {}

#[derive(QueryId, Copy, Clone, Debug)]
pub struct BetweenFrame<Kind, Start, End, Exclusion = NoExclusion> {
    kind: Kind,
    start: Start,
    end: End,
    exclusion: Exclusion,
}

impl<Kind, Start, End, Exclusion, DB> QueryFragment<DB>
    for BetweenFrame<Kind, Start, End, Exclusion>
where
    Kind: QueryFragment<DB>,
    Start: QueryFragment<DB>,
    End: QueryFragment<DB>,
    Exclusion: QueryFragment<DB>,
    DB: crate::backend::Backend,
{
    fn walk_ast<'b>(
        &'b self,
        mut pass: crate::query_builder::AstPass<'_, 'b, DB>,
    ) -> crate::QueryResult<()> {
        self.kind.walk_ast(pass.reborrow())?;
        pass.push_sql(" BETWEEN ");
        self.start.walk_ast(pass.reborrow())?;
        pass.push_sql(" AND ");
        self.end.walk_ast(pass.reborrow())?;
        self.exclusion.walk_ast(pass.reborrow())?;
        Ok(())
    }
}

impl<Kind, Start, End, Exclusion> FrameClauseExpression
    for BetweenFrame<Kind, Start, End, Exclusion>
{
}

pub trait FrameClauseDslHelper: Sized {}

/// Construct a frame clause for window functions
pub trait FrameClauseDsl: FrameClauseDslHelper {
    /// Construct a frame clause with a starting bound
    fn start_with<E>(self, start: E) -> StartFrame<Self, E>
    where
        E: FrameClauseStartBound,
    {
        StartFrame {
            kind: self,
            start,
            exclusion: NoExclusion,
        }
    }

    /// Construct a frame clause with a starting bound and an exclusion condition
    fn start_with_exclusion<E1, E2>(self, start: E1, exclusion: E2) -> StartFrame<Self, E1, E2>
    where
        E1: FrameClauseStartBound,
        E2: FrameCauseExclusion,
    {
        StartFrame {
            kind: self,
            start,
            exclusion,
        }
    }

    /// Construct a between frame clause with a starting and end bound
    fn between<E1, E2>(self, start: E1, end: E2) -> BetweenFrame<Self, E1, E2>
    where
        E1: FrameClauseStartBound,
        E2: FrameClauseEndBound,
    {
        BetweenFrame {
            kind: self,
            start,
            end,
            exclusion: NoExclusion,
        }
    }

    /// Construct a between frame clause with a starting and end bound  with an exclusion condition
    fn between_with_exclusion<E1, E2, E3>(
        self,
        start: E1,
        end: E2,
        exclusion: E3,
    ) -> BetweenFrame<Self, E1, E2, E3>
    where
        E1: FrameClauseStartBound,
        E2: FrameClauseEndBound,
        E3: FrameCauseExclusion,
    {
        BetweenFrame {
            kind: self,
            start,
            end,
            exclusion,
        }
    }
}

impl<T> FrameClauseDsl for T where T: FrameClauseDslHelper {}

impl FrameClauseDslHelper for Range {}
impl FrameClauseDslHelper for Rows {}
impl FrameClauseDslHelper for Groups {}
