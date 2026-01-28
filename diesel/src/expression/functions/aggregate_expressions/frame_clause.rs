use crate::backend::sql_dialect;
use crate::query_builder::{QueryFragment, QueryId};

use super::aggregate_filter::NoFilter;
use super::aggregate_order::{NoOrder, Order};
use super::over_clause::{NoWindow, OverClause, ValidAggregateFilterForWindow};
use super::partition_by::NoPartition;
use super::prefix::NoPrefix;
use super::{AggregateExpression, IsWindowFunction};

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
    ($(#[doc = $doc: literal])* $name: ident, $kind: expr) => {
        #[derive(QueryId, Clone, Copy, Debug)]
        $(#[doc = $doc])*
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

macro_rules! simple_frame_expr_with_bound {
    ($(#[doc = $doc:literal])* $name: ident, $option: ident, $bound: ty, $kind: expr) => {
        #[derive(QueryId, Clone, Copy, Debug)]
        $(#[doc = $doc])*
        pub struct $name;

        impl<DB> QueryFragment<DB> for $name
        where
            DB: crate::backend::Backend,
            Self: QueryFragment<DB, DB::$option>,
        {
            fn walk_ast<'b>(
                &'b self,
                pass: crate::query_builder::AstPass<'_, 'b, DB>,
            ) -> crate::QueryResult<()> {
                <Self as QueryFragment<DB, DB::$option>>::walk_ast(self, pass)
            }
        }
        impl<DB> QueryFragment<DB, $bound> for $name
        where
            DB: crate::backend::Backend<$option = $bound>,
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
simple_frame_expr!(
    /// Range frame mode
    ///
    /// Requires to set a `ORDER BY` clause set via `window_order`
    /// for the current window function call
    Range, " RANGE ");
simple_frame_expr!(
    /// Rows frame mode
    ///
    /// This options specifies that the offset starts/ends
    /// a certain number of rows before/after the current row
    Rows, " ROWS ");
simple_frame_expr_with_bound!(
    /// Groups frame mode
    ///
    /// This options specifies that the offset starts/ends
    /// a certain number of peer groups before or after
    /// the current row. A peer group is a set of rows that are
    /// equivalent in the `ORDER BY` ordering
    ///
    /// There must be a `ORDER BY` clause set via `window_order`
    /// for the current window function call
    Groups,
    WindowFrameClauseGroupSupport,
    sql_dialect::window_frame_clause_group_support::IsoGroupWindowFrameUnit,
    " GROUPS "
);

// start & end
simple_frame_expr!(
    /// A `UNBOUNDED PRECEDING` frame bound
    ///
    /// This bound specifies that the frame starts with
    /// the first row of the partition
    ///
    /// This option can be used as frame start
    UnboundedPreceding, "UNBOUNDED PRECEDING ");
simple_frame_expr!(
    /// A `CURRENT ROW` frame bound
    ///
    /// For the `RANGE` and `GROUP` frame mode this bound
    /// specifies that the current frame starts with the current
    /// rows first peer groups row. A peer group is defined as a
    /// set of rows with an equivalent `ORDER BY` ordering.
    ///
    /// For the `ROWS` mode `CURRENT ROW` simply means the current
    /// row
    ///
    /// This option can be used as frame start and end
    CurrentRow, "CURRENT ROW ");
simple_frame_expr!(
    /// A `UNBOUNDED FOLLOWING` frame bound
    ///
    /// This bound specifies that the frame ends with
    /// the last row of the partition
    ///
    /// This option can be used as frame end
    UnboundedFollowing, "UNBOUNDED FOLLOWING ");

// exclusion
simple_frame_expr_with_bound!(
    /// Exclude the current row from the window frame
    ExcludeCurrentRow,
    WindowFrameExclusionSupport,
    sql_dialect::window_frame_exclusion_support::FrameExclusionSupport,
    "EXCLUDE CURRENT ROW "
);
simple_frame_expr_with_bound!(
    /// Exclude the current peer group from the window frame
    ///
    /// A peer group is a set of rows with an equivalent `ORDER BY`
    /// ordering
    ExcludeGroup,
    WindowFrameExclusionSupport,
    sql_dialect::window_frame_exclusion_support::FrameExclusionSupport,
    "EXCLUDE GROUP "
);
simple_frame_expr_with_bound!(
    /// Exclude any peers, but not the current row from the window frame
    ///
    /// This excludes any row with an equivalent `ORDER BY` ordering
    /// to the current row from the frame window
    ExcludeTies,
    WindowFrameExclusionSupport,
    sql_dialect::window_frame_exclusion_support::FrameExclusionSupport,
    "EXCLUDE TIES "
);
simple_frame_expr_with_bound!(
    /// Exclude no rows from the frame windows
    ///
    /// This is the default behaviour if not specified
    ExcludeNoOthers,
    WindowFrameExclusionSupport,
    sql_dialect::window_frame_exclusion_support::FrameExclusionSupport,
    "EXCLUDE NO OTHERS "
);

/// A preceding frame clause expression with a fixed offset
///
/// Can be constructed via [`FrameBoundDsl::preceding`]
#[derive(Clone, Copy, Debug)]
pub struct OffsetPreceding<T = u64>(T);

// manual impl as the derive makes this dependent on a `T: QueryId` impl
// which is wrong
impl QueryId for OffsetPreceding {
    type QueryId = ();

    const HAS_STATIC_QUERY_ID: bool = true;
}

impl<DB> QueryFragment<DB> for OffsetPreceding
where
    DB: crate::backend::Backend,
{
    fn walk_ast<'b>(
        &'b self,
        mut pass: crate::query_builder::AstPass<'_, 'b, DB>,
    ) -> crate::QueryResult<()> {
        // we cannot use binds here as mysql doesn't support it :(
        // at least it's safe to just insert a number here
        pass.push_sql(&self.0.to_string());
        pass.push_sql(" PRECEDING ");
        Ok(())
    }
}

/// A following frame clause expression with a fixed offset
///
/// Can be constructed via [`FrameBoundDsl::following`]
#[derive(Clone, Copy, Debug)]
pub struct OffsetFollowing<I = u64>(I);

// manual impl as the derive makes this dependent on a `T: QueryId` impl
// which is wrong
impl QueryId for OffsetFollowing {
    type QueryId = ();

    const HAS_STATIC_QUERY_ID: bool = true;
}

impl<DB> QueryFragment<DB> for OffsetFollowing
where
    DB: crate::backend::Backend,
{
    fn walk_ast<'b>(
        &'b self,
        mut pass: crate::query_builder::AstPass<'_, 'b, DB>,
    ) -> crate::QueryResult<()> {
        // we cannot use binds here as mysql doesn't support it :(
        // at least it's safe to just insert a number here
        pass.push_sql(&self.0.to_string());
        pass.push_sql(" FOLLOWING ");
        Ok(())
    }
}

pub trait FrameDsl<F> {
    type Output;

    fn frame(self, expr: F) -> Self::Output;
}

#[diagnostic::on_unimplemented(
    message = "`Groups` frame clauses require a ordered window function",
    note = "call `.window_order(some_column)` first"
)]
pub trait ValidFrameClause<O> {}

impl<O, Kind, Start, End, Exclusion> ValidFrameClause<O>
    for BetweenFrame<Kind, Start, End, Exclusion>
where
    Kind: ValidFrameClause<O>,
{
}
impl<O, Kind, Start, Exclusion> ValidFrameClause<O> for StartFrame<Kind, Start, Exclusion> where
    Kind: ValidFrameClause<O>
{
}
impl<O> ValidFrameClause<O> for Rows {}
impl<O> ValidFrameClause<O> for Range {}
impl<O> ValidFrameClause<Order<O, true>> for Groups {}

impl<E, Fn, Filter, Frame, Partition, Order> FrameDsl<E>
    for AggregateExpression<Fn, NoPrefix, NoOrder, Filter, OverClause<Partition, Order, Frame>>
where
    E: FrameClauseExpression,
    E: ValidFrameClause<Order>,
    Filter: ValidAggregateFilterForWindow<Fn, OverClause<Partition, Order, FrameClause<E>>>,
{
    type Output = AggregateExpression<
        Fn,
        NoPrefix,
        NoOrder,
        Filter,
        OverClause<Partition, Order, FrameClause<E>>,
    >;

    fn frame(self, expr: E) -> Self::Output {
        AggregateExpression {
            prefix: self.prefix,
            function: self.function,
            order: self.order,
            filter: self.filter,
            window: OverClause {
                partition_by: self.window.partition_by,
                order: self.window.order,
                frame_clause: FrameClause(expr),
            },
        }
    }
}

impl<E, Fn, Filter> FrameDsl<E> for AggregateExpression<Fn, NoPrefix, NoOrder, Filter, NoWindow>
where
    E: FrameClauseExpression,
    E: ValidFrameClause<NoOrder>,
    Filter: ValidAggregateFilterForWindow<Fn, OverClause<NoPartition, NoOrder, FrameClause<E>>>,
{
    type Output = AggregateExpression<
        Fn,
        NoPrefix,
        NoOrder,
        Filter,
        OverClause<NoPartition, NoOrder, FrameClause<E>>,
    >;

    fn frame(self, expr: E) -> Self::Output {
        AggregateExpression {
            prefix: self.prefix,
            function: self.function,
            order: self.order,
            filter: self.filter,
            window: OverClause {
                partition_by: NoPartition,
                order: NoOrder,
                frame_clause: FrameClause(expr),
            },
        }
    }
}

impl<E, Fn> FrameDsl<E> for Fn
where
    Fn: IsWindowFunction,
    E: FrameClauseExpression,
    E: ValidFrameClause<NoOrder>,
{
    type Output = AggregateExpression<
        Fn,
        NoPrefix,
        NoOrder,
        NoFilter,
        OverClause<NoPartition, NoOrder, FrameClause<E>>,
    >;

    fn frame(self, expr: E) -> Self::Output {
        AggregateExpression {
            prefix: NoPrefix,
            function: self,
            order: NoOrder,
            filter: NoFilter,
            window: OverClause {
                partition_by: NoPartition,
                order: NoOrder,
                frame_clause: FrameClause(expr),
            },
        }
    }
}

pub trait FrameClauseExpression {}

/// A marker trait for possible start frame expressions
///
/// See the list of types implementing this trait to understand
/// what can be used in this position
pub trait FrameClauseStartBound: Sealed {}

/// A marker trait for possible end frame expressions
///
/// See the list of types implementing this trait to understand
/// what can be used in this position
pub trait FrameClauseEndBound: Sealed {}

impl FrameClauseEndBound for UnboundedFollowing {}
impl Sealed for UnboundedFollowing {}
impl FrameClauseStartBound for UnboundedPreceding {}
impl Sealed for UnboundedPreceding {}
impl FrameClauseEndBound for CurrentRow {}
impl FrameClauseStartBound for CurrentRow {}
impl Sealed for CurrentRow {}
impl FrameClauseEndBound for OffsetFollowing {}
impl Sealed for OffsetFollowing {}
impl FrameClauseStartBound for OffsetPreceding {}
impl Sealed for OffsetPreceding {}

/// A marker trait for possible frame exclusion expressions
///
/// See the list of types implementing this trait to understand
/// what can be used in this position
pub trait FrameClauseExclusion: Sealed {}

impl FrameClauseExclusion for ExcludeGroup {}
impl Sealed for ExcludeGroup {}
impl FrameClauseExclusion for ExcludeNoOthers {}
impl Sealed for ExcludeNoOthers {}
impl FrameClauseExclusion for ExcludeTies {}
impl Sealed for ExcludeTies {}
impl FrameClauseExclusion for ExcludeCurrentRow {}
impl Sealed for ExcludeCurrentRow {}

/// Construct a frame clause for window functions from an integer
pub trait FrameBoundDsl {
    /// Use the preceding frame clause specification
    fn preceding(self) -> OffsetPreceding;

    /// Use the following frame clause specification
    fn following(self) -> OffsetFollowing;
}

impl FrameBoundDsl for u64 {
    fn preceding(self) -> OffsetPreceding {
        OffsetPreceding(self)
    }

    fn following(self) -> OffsetFollowing {
        OffsetFollowing(self)
    }
}
// TODO: We might want to implement
// it for datetime and date intervals?
// The postgres documentation indicates that
// something like `RANGE BETWEEN '1 day' PRECEDING AND '10 days' FOLLOWING`
// is valid

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
    fn frame_start_with<E>(self, start: E) -> super::dsl::FrameStartWith<Self, E>
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
    fn frame_start_with_exclusion<E1, E2>(
        self,
        start: E1,
        exclusion: E2,
    ) -> super::dsl::FrameStartWithExclusion<Self, E1, E2>
    where
        E1: FrameClauseStartBound,
        E2: FrameClauseExclusion,
    {
        StartFrame {
            kind: self,
            start,
            exclusion,
        }
    }

    /// Construct a between frame clause with a starting and end bound
    fn frame_between<E1, E2>(self, start: E1, end: E2) -> super::dsl::FrameBetween<Self, E1, E2>
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
    fn frame_between_with_exclusion<E1, E2, E3>(
        self,
        start: E1,
        end: E2,
        exclusion: E3,
    ) -> super::dsl::FrameBetweenWithExclusion<Self, E1, E2, E3>
    where
        E1: FrameClauseStartBound,
        E2: FrameClauseEndBound,
        E3: FrameClauseExclusion,
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

pub trait Sealed {}
