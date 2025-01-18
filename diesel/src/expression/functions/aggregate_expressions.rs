use crate::backend::Backend;
use crate::expression::{AsExpression, ValidGrouping};
use crate::query_builder::{AstPass, NotSpecialized, QueryFragment, QueryId};
use crate::sql_types::Bool;
use crate::{AppearsOnTable, Expression, QueryResult, SelectableExpression};

macro_rules! empty_clause {
    ($name: ident) => {
        #[derive(Debug, Clone, Copy, QueryId)]
        pub struct $name;

        impl<DB> crate::query_builder::QueryFragment<DB> for $name
        where
            DB: crate::backend::Backend + crate::backend::DieselReserveSpecialization,
        {
            fn walk_ast<'b>(
                &'b self,
                _pass: crate::query_builder::AstPass<'_, 'b, DB>,
            ) -> crate::QueryResult<()> {
                Ok(())
            }
        }
    };
}

mod aggregate_filter;
mod aggregate_order;
pub(crate) mod frame_clause;
mod over_clause;
mod partition_by;
mod prefix;
mod within_group;

use self::aggregate_filter::{FilterDsl, NoFilter};
use self::aggregate_order::{NoOrder, OrderAggregateDsl, OrderWindowDsl};
use self::frame_clause::{FrameDsl, NoFrame};
pub use self::over_clause::OverClause;
use self::over_clause::{NoWindow, OverDsl};
use self::partition_by::PartitionByDsl;
use self::prefix::{All, AllDsl, DistinctDsl, NoPrefix};
use self::within_group::{NoWithin, WithinGroupDsl};

#[derive(QueryId, Debug)]
pub struct AggregateExpression<
    Fn,
    Prefix = NoPrefix,
    Order = NoOrder,
    Filter = NoFilter,
    Within = NoWithin,
    Window = NoWindow,
> {
    prefix: Prefix,
    function: Fn,
    order: Order,
    filter: Filter,
    within_group: Within,
    window: Window,
}

impl<Fn, Prefix, Order, Filter, Within, Window, DB> QueryFragment<DB>
    for AggregateExpression<Fn, Prefix, Order, Filter, Within, Window>
where
    DB: crate::backend::Backend + crate::backend::DieselReserveSpecialization,
    Fn: FunctionFragment<DB>,
    Prefix: QueryFragment<DB>,
    Order: QueryFragment<DB>,
    Filter: QueryFragment<DB>,
    Within: QueryFragment<DB>,
    Window: QueryFragment<DB> + WindowFunctionFragment<Fn, DB>,
{
    fn walk_ast<'b>(&'b self, mut pass: AstPass<'_, 'b, DB>) -> QueryResult<()> {
        pass.push_sql(Fn::FUNCTION_NAME);
        pass.push_sql("(");
        self.prefix.walk_ast(pass.reborrow())?;
        self.function.walk_arguments(pass.reborrow())?;
        self.order.walk_ast(pass.reborrow())?;
        pass.push_sql(")");
        self.within_group.walk_ast(pass.reborrow())?;
        self.filter.walk_ast(pass.reborrow())?;
        self.window.walk_ast(pass.reborrow())?;
        Ok(())
    }
}

impl<Fn, Prefix, Order, Filter, Within, GB> ValidGrouping<GB>
    for AggregateExpression<Fn, Prefix, Order, Filter, Within>
where
    Fn: ValidGrouping<GB>,
{
    type IsAggregate = <Fn as ValidGrouping<GB>>::IsAggregate;
}

impl<Fn, Prefix, Order, Filter, Within, GB, Partition, WindowOrder, Frame> ValidGrouping<GB>
    for AggregateExpression<
        Fn,
        Prefix,
        Order,
        Filter,
        Within,
        OverClause<Partition, WindowOrder, Frame>,
    >
where
    Fn: IsWindowFunction,
    Fn::ArgTypes: ValidGrouping<GB>,
{
    // not sure about that, check this
    type IsAggregate = <Fn::ArgTypes as ValidGrouping<GB>>::IsAggregate;
}

impl<Fn, Prefix, Order, Filter, Within, Window> Expression
    for AggregateExpression<Fn, Prefix, Order, Filter, Within, Window>
where
    Fn: Expression,
{
    type SqlType = <Fn as Expression>::SqlType;
}

impl<Fn, Prefix, Order, Filter, Within, Window, QS> AppearsOnTable<QS>
    for AggregateExpression<Fn, Prefix, Order, Filter, Within, Window>
where
    Self: Expression,
    Fn: AppearsOnTable<QS>,
{
}

impl<Fn, Prefix, Order, Filter, Within, Window, QS> SelectableExpression<QS>
    for AggregateExpression<Fn, Prefix, Order, Filter, Within, Window>
where
    Self: Expression,
    Fn: SelectableExpression<QS>,
{
}

/// A helper marker trait that this function is a window function
/// This is only used to provide the gate the `WindowExpressionMethods`
/// trait onto, not to check if the construct is valid for a given backend
/// This check is postponed to building the query via `QueryFragment`
/// (We have access to the DB type there)
pub trait IsWindowFunction {
    /// A tuple of all arg types
    type ArgTypes;
}

/// A helper marker trait that this function is a valid window function
/// for the given backend
/// this trait is used to transport information that
/// a certain function can be used as window function for a specific
/// backend
/// We allow to specialize this function for different SQL dialects
pub trait WindowFunctionFragment<Fn, DB: Backend, SP = NotSpecialized> {}

/// A helper marker trait that this function as a aggregate function
/// This is only used to provide the gate the `AggregateExpressionMethods`
/// trait onto, not to check if the construct is valid for a given backend
/// This check is postponed to building the query via `QueryFragment`
/// (We have access to the DB type there)
pub trait IsAggregateFunction {}

/// A specialized QueryFragment helper trait that allows us to walk the function name
/// and the function arguments in seperate steps
pub trait FunctionFragment<DB: Backend> {
    /// The name of the sql function
    const FUNCTION_NAME: &'static str;

    /// Walk the function argument part (everything between ())
    fn walk_arguments<'b>(&'b self, pass: AstPass<'_, 'b, DB>) -> QueryResult<()>;
}

// TODO: write helper types for all functions
// TODO: write doc tests for all functions
/// Expression methods to build aggregate function expressions
pub trait AggregateExpressionMethods: Sized {
    /// `DISTINCT` modifier
    fn distinct(self) -> Self::Output
    where
        Self: DistinctDsl,
    {
        <Self as DistinctDsl>::distinct(self)
    }

    /// `ALL` modifier
    fn all(self) -> Self::Output
    where
        Self: AllDsl,
    {
        <Self as AllDsl>::all(self)
    }

    /// Add an aggregate filter
    fn filter_aggregate<P>(self, f: P) -> Self::Output
    where
        P: AsExpression<Bool>,
        Self: FilterDsl<P::Expression>,
    {
        <Self as FilterDsl<P::Expression>>::filter(self, f.as_expression())
    }

    /// Add an aggregate order
    fn order_aggregate<O>(self, o: O) -> Self::Output
    where
        Self: OrderAggregateDsl<O>,
    {
        <Self as OrderAggregateDsl<O>>::order(self, o)
    }

    // todo: restrict this to order set aggregates
    // (we don't have any in diesel yet)
    #[doc(hidden)] // for now
    fn within_group<O>(self, o: O) -> Self::Output
    where
        Self: WithinGroupDsl<O>,
    {
        <Self as WithinGroupDsl<O>>::within_group(self, o)
    }
}

impl<T> AggregateExpressionMethods for T {}

/// Methods to construct a window function call
pub trait WindowExpressionMethods: Sized {
    /// Turn a function call into a window function call
    fn over(self) -> Self::Output
    where
        Self: OverDsl,
    {
        <Self as OverDsl>::over(self)
    }

    /// Add a filter to the current window function
    // todo: do we want `or_filter` as well?
    fn filter_window<P>(self, f: P) -> Self::Output
    where
        P: AsExpression<Bool>,
        Self: FilterDsl<P::Expression>,
    {
        <Self as FilterDsl<P::Expression>>::filter(self, f.as_expression())
    }

    /// Add a partition clause to the current window function
    fn partition_by<E>(self, expr: E) -> Self::Output
    where
        Self: PartitionByDsl<E>,
    {
        <Self as PartitionByDsl<E>>::partition_by(self, expr)
    }

    /// Add a order clause to the current window function
    fn window_order<E>(self, expr: E) -> Self::Output
    where
        Self: OrderWindowDsl<E>,
    {
        <Self as OrderWindowDsl<E>>::order(self, expr)
    }

    /// Add a frame clause to the current window function
    fn frame_by<E>(self, expr: E) -> Self::Output
    where
        Self: FrameDsl<E>,
    {
        <Self as FrameDsl<E>>::frame(self, expr)
    }
}

impl<T> WindowExpressionMethods for T {}
