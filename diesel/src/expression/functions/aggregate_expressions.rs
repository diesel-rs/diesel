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

use self::aggregate_filter::{FilterDsl, NoFilter};
pub use self::aggregate_order::Order;
use self::aggregate_order::{NoOrder, OrderAggregateDsl, OrderWindowDsl};
use self::frame_clause::{FrameDsl, NoFrame};
pub use self::over_clause::OverClause;
use self::over_clause::{NoWindow, OverDsl};
use self::partition_by::PartitionByDsl;
use self::prefix::{AllDsl, DistinctDsl, NoPrefix};

#[derive(QueryId, Debug)]
pub struct AggregateExpression<
    Fn,
    Prefix = NoPrefix,
    Order = NoOrder,
    Filter = NoFilter,
    Window = NoWindow,
> {
    prefix: Prefix,
    function: Fn,
    order: Order,
    filter: Filter,
    window: Window,
}

impl<Fn, Prefix, Order, Filter, Window, DB> QueryFragment<DB>
    for AggregateExpression<Fn, Prefix, Order, Filter, Window>
where
    DB: crate::backend::Backend + crate::backend::DieselReserveSpecialization,
    Fn: FunctionFragment<DB>,
    Prefix: QueryFragment<DB>,
    Order: QueryFragment<DB>,
    Filter: QueryFragment<DB>,
    Window: QueryFragment<DB> + WindowFunctionFragment<Fn, DB>,
{
    fn walk_ast<'b>(&'b self, mut pass: AstPass<'_, 'b, DB>) -> QueryResult<()> {
        pass.push_sql(Fn::FUNCTION_NAME);
        pass.push_sql("(");
        self.prefix.walk_ast(pass.reborrow())?;
        self.function.walk_arguments(pass.reborrow())?;
        self.order.walk_ast(pass.reborrow())?;
        pass.push_sql(")");
        self.filter.walk_ast(pass.reborrow())?;
        self.window.walk_ast(pass.reborrow())?;
        Ok(())
    }
}

impl<Fn, Prefix, Order, Filter, GB> ValidGrouping<GB>
    for AggregateExpression<Fn, Prefix, Order, Filter>
where
    Fn: ValidGrouping<GB>,
{
    type IsAggregate = <Fn as ValidGrouping<GB>>::IsAggregate;
}

impl<Fn, Prefix, Order, Filter, GB, Partition, WindowOrder, Frame> ValidGrouping<GB>
    for AggregateExpression<Fn, Prefix, Order, Filter, OverClause<Partition, WindowOrder, Frame>>
where
    Fn: IsWindowFunction,
    Fn::ArgTypes: ValidGrouping<GB>,
{
    type IsAggregate = <Fn::ArgTypes as ValidGrouping<GB>>::IsAggregate;
}

impl<Fn, Prefix, Order, Filter, Window> Expression
    for AggregateExpression<Fn, Prefix, Order, Filter, Window>
where
    Fn: Expression,
{
    type SqlType = <Fn as Expression>::SqlType;
}

impl<Fn, Prefix, Order, Filter, Window, QS> AppearsOnTable<QS>
    for AggregateExpression<Fn, Prefix, Order, Filter, Window>
where
    Self: Expression,
    Fn: AppearsOnTable<QS>,
{
}

impl<Fn, Prefix, Order, Filter, Window, QS> SelectableExpression<QS>
    for AggregateExpression<Fn, Prefix, Order, Filter, Window>
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
#[diagnostic::on_unimplemented(
    message = "{Self} is not a window function",
    label = "remove this function call to use `{Self}` as normal SQL function",
    note = "try removing any method call to `WindowExpressionMethods` and use it as normal SQL function"
)]
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
/// and the function arguments in separate steps
pub trait FunctionFragment<DB: Backend> {
    /// The name of the sql function
    const FUNCTION_NAME: &'static str;

    /// Walk the function argument part (everything between ())
    fn walk_arguments<'b>(&'b self, pass: AstPass<'_, 'b, DB>) -> QueryResult<()>;
}

/// Expression methods to build aggregate function expressions
pub trait AggregateExpressionMethods: Sized {
    /// `DISTINCT` modifier for aggregate functions
    ///
    /// This modifies the aggregate function call to only
    /// include distinct items
    ///
    /// # Example
    ///
    /// ```rust
    /// # include!("../../doctest_setup.rs");
    /// #
    /// # fn main() {
    /// #     run_test().unwrap();
    /// # }
    /// #
    /// # fn run_test() -> QueryResult<()> {
    /// #     use schema::posts::dsl::*;
    /// #     use diesel::dsl;
    /// #     let connection = &mut establish_connection();
    /// let without_distinct = posts
    ///     .select(dsl::count(user_id))
    ///     .get_result::<i64>(connection)?;
    /// let with_distinct = posts
    ///     .select(dsl::count(user_id).aggregate_distinct())
    ///     .get_result::<i64>(connection)?;
    ///
    /// assert_eq!(3, without_distinct);
    /// assert_eq!(2, with_distinct);
    /// #     Ok(())
    /// # }
    /// ```
    fn aggregate_distinct(self) -> self::dsl::AggregateDistinct<Self>
    where
        Self: DistinctDsl,
    {
        <Self as DistinctDsl>::distinct(self)
    }

    /// `ALL` modifier for aggregate functions
    ///
    /// This modifies the aggregate function call to include
    /// all items. This is the default behaviour.
    ///
    /// # Example
    ///
    /// ```rust
    /// # include!("../../doctest_setup.rs");
    /// #
    /// # fn main() {
    /// #     run_test().unwrap();
    /// # }
    /// #
    /// # fn run_test() -> QueryResult<()> {
    /// #     use schema::posts::dsl::*;
    /// #     use diesel::dsl;
    /// #     let connection = &mut establish_connection();
    /// let without_all = posts
    ///     .select(dsl::count(user_id))
    ///     .get_result::<i64>(connection)?;
    /// let with_all = posts
    ///     .select(dsl::count(user_id).aggregate_all())
    ///     .get_result::<i64>(connection)?;
    ///
    /// assert_eq!(3, without_all);
    /// assert_eq!(3, with_all);
    /// #     Ok(())
    /// # }
    /// ```
    fn aggregate_all(self) -> self::dsl::AggregateAll<Self>
    where
        Self: AllDsl,
    {
        <Self as AllDsl>::all(self)
    }

    /// Add an aggregate function filter
    ///
    /// This function modifies an aggregate function
    /// call to use only items matching the provided
    /// filter
    ///
    /// # Example
    ///
    /// ```rust
    /// # include!("../../doctest_setup.rs");
    /// #
    /// # fn main() {
    /// #     #[cfg(not(feature = "mysql"))]
    /// #     run_test().unwrap();
    /// # }
    /// #
    /// # #[cfg(not(feature = "mysql"))]
    /// # fn run_test() -> QueryResult<()> {
    /// #     use schema::posts::dsl::*;
    /// #     use diesel::dsl;
    /// #     let connection = &mut establish_connection();
    /// let without_filter = posts
    ///     .select(dsl::count(user_id))
    ///     .get_result::<i64>(connection)?;
    /// let with_filter = posts
    ///     .select(dsl::count(user_id).aggregate_filter(title.like("%first post%")))
    ///     .get_result::<i64>(connection)?;
    ///
    /// assert_eq!(3, without_filter);
    /// assert_eq!(2, with_filter);
    /// #     Ok(())
    /// # }
    /// ```
    fn aggregate_filter<P>(self, f: P) -> self::dsl::AggregateFilter<Self, P>
    where
        P: AsExpression<Bool>,
        Self: FilterDsl<P::Expression>,
    {
        <Self as FilterDsl<P::Expression>>::filter(self, f.as_expression())
    }

    /// Add an aggregate function order
    ///
    /// This function orders the items passed into an
    /// aggregate function
    ///
    /// For sqlite this is only supported starting with SQLite 3.44
    ///
    /// # Example
    ///
    /// ```rust
    /// # include!("../../doctest_setup.rs");
    /// #
    /// # fn main() {
    /// #     #[cfg(not(feature = "mysql"))]
    /// #     run_test().unwrap();
    /// # }
    /// #
    /// # #[cfg(not(feature = "mysql"))]
    /// # fn run_test() -> QueryResult<()> {
    /// #     use schema::posts::dsl::*;
    /// #     use diesel::dsl;
    /// #     let connection = &mut establish_connection();
    /// #     #[cfg(feature = "sqlite")]
    /// #     assert_version!(connection, 3, 44, 0);
    /// // This example is not meaningful yet,
    /// // modify it as soon as we support more
    /// // meaningful functions here
    /// let res = posts
    ///     .select(dsl::count(user_id).aggregate_order(title))
    ///     .get_result::<i64>(connection)?;
    /// assert_eq!(3, res);
    /// #     Ok(())
    /// # }
    /// ```
    fn aggregate_order<O>(self, o: O) -> self::dsl::AggregateOrder<Self, O>
    where
        Self: OrderAggregateDsl<O>,
    {
        <Self as OrderAggregateDsl<O>>::order(self, o)
    }
}

impl<T> AggregateExpressionMethods for T {}

/// Methods to construct a window function call
pub trait WindowExpressionMethods: Sized {
    /// Turn a function call into a window function call
    ///
    /// This function turns a ordinary SQL function call
    /// into a window function call by adding an empty `OVER ()`
    /// clause
    ///
    /// # Example
    ///
    /// ```rust
    /// # include!("../../doctest_setup.rs");
    /// #
    /// # fn main() {
    /// #     run_test().unwrap();
    /// # }
    /// #
    /// # fn run_test() -> QueryResult<()> {
    /// #     use schema::posts::dsl::*;
    /// #     use diesel::dsl;
    /// #     let connection = &mut establish_connection();
    /// let res = posts
    ///     .select(dsl::count(user_id).over())
    ///     .load::<i64>(connection)?;
    /// assert_eq!(vec![3, 3, 3], res);
    /// #     Ok(())
    /// # }
    /// ```
    fn over(self) -> self::dsl::Over<Self>
    where
        Self: OverDsl,
    {
        <Self as OverDsl>::over(self)
    }

    /// Add a filter to the current window function
    ///
    ///
    /// # Example
    ///
    /// ```rust
    /// # include!("../../doctest_setup.rs");
    /// #
    /// # fn main() {
    /// #     #[cfg(not(feature = "mysql"))]
    /// #     run_test().unwrap();
    /// # }
    /// #
    /// # #[cfg(not(feature = "mysql"))]
    /// # fn run_test() -> QueryResult<()> {
    /// #     use schema::posts::dsl::*;
    /// #     use diesel::dsl;
    /// #     let connection = &mut establish_connection();
    /// let res = posts
    ///     .select(dsl::count(user_id).window_filter(user_id.eq(1)))
    ///     .load::<i64>(connection)?;
    /// assert_eq!(vec![2], res);
    /// #     Ok(())
    /// # }
    /// ```
    fn window_filter<P>(self, f: P) -> self::dsl::WindowFilter<Self, P>
    where
        P: AsExpression<Bool>,
        Self: FilterDsl<P::Expression>,
    {
        <Self as FilterDsl<P::Expression>>::filter(self, f.as_expression())
    }

    /// Add a partition clause to the current window function
    ///
    /// This function adds a `PARTITION BY` clause to your window function call
    ///
    /// # Example
    ///
    /// ```rust
    /// # include!("../../doctest_setup.rs");
    /// #
    /// # fn main() {
    /// #     run_test().unwrap();
    /// # }
    /// #
    /// # fn run_test() -> QueryResult<()> {
    /// #     use schema::posts::dsl::*;
    /// #     use diesel::dsl;
    /// #     let connection = &mut establish_connection();
    /// let res = posts
    ///     .select(dsl::count(user_id).partition_by(user_id))
    ///     .load::<i64>(connection)?;
    /// assert_eq!(vec![2, 2, 1], res);
    /// #     Ok(())
    /// # }
    /// ```
    fn partition_by<E>(self, expr: E) -> self::dsl::PartitionBy<Self, E>
    where
        Self: PartitionByDsl<E>,
    {
        <Self as PartitionByDsl<E>>::partition_by(self, expr)
    }

    /// Add a order clause to the current window function
    ///
    /// Add a `ORDER BY` clause to your window function call
    ///
    /// # Example
    ///
    /// ```rust
    /// # include!("../../doctest_setup.rs");
    /// #
    /// # fn main() {
    /// #     run_test().unwrap();
    /// # }
    /// #
    /// # fn run_test() -> QueryResult<()> {
    /// #     use schema::posts::dsl::*;
    /// #     use diesel::dsl;
    /// #     let connection = &mut establish_connection();
    /// let res = posts
    ///     .select(dsl::first_value(user_id).window_order(title))
    ///     .load::<i32>(connection)?;
    /// assert_eq!(vec![1, 1, 1], res);
    /// #     Ok(())
    /// # }
    /// ```
    fn window_order<E>(self, expr: E) -> self::dsl::WindowOrder<Self, E>
    where
        Self: OrderWindowDsl<E>,
    {
        <Self as OrderWindowDsl<E>>::order(self, expr)
    }

    /// Add a frame clause to the current window function
    ///
    /// This function adds a frame clause to your window function call.
    /// Accepts the following items:
    ///
    /// * [`dsl::frame::Groups`](crate::dsl::frame::Groups)
    /// * [`dsl::frame::Rows`](crate::dsl::frame::Rows)
    /// * [`dsl::frame::Range`](crate::dsl::frame::Range)
    ///
    /// # Example
    ///
    /// ```rust
    /// # include!("../../doctest_setup.rs");
    /// #
    /// # fn main() {
    /// #     run_test().unwrap();
    /// # }
    /// #
    /// # fn run_test() -> QueryResult<()> {
    /// #     use schema::posts::dsl::*;
    /// #     use diesel::dsl;
    /// #     let connection = &mut establish_connection();
    /// let res = posts
    ///     .select(
    ///         dsl::count(user_id).frame_by(dsl::frame::Rows.frame_start_with(dsl::frame::CurrentRow)),
    ///     )
    ///     .load::<i64>(connection)?;
    /// assert_eq!(vec![1, 1, 1], res);
    /// #     Ok(())
    /// # }
    /// ```
    fn frame_by<E>(self, expr: E) -> self::dsl::FrameBy<Self, E>
    where
        Self: FrameDsl<E>,
    {
        <Self as FrameDsl<E>>::frame(self, expr)
    }
}

impl<T> WindowExpressionMethods for T {}

pub(super) mod dsl {
    #[cfg(doc)]
    use super::frame_clause::{FrameBoundDsl, FrameClauseDsl};
    use super::*;

    /// Return type of [`WindowExpressionMethods::over`]
    pub type Over<Fn> = <Fn as OverDsl>::Output;

    /// Return type of [`WindowExpressionMethods::window_filter`]
    pub type WindowFilter<Fn, P> = <Fn as FilterDsl<crate::dsl::AsExprOf<P, Bool>>>::Output;

    /// Return type of [`WindowExpressionMethods::partition_by`]
    pub type PartitionBy<Fn, E> = <Fn as PartitionByDsl<E>>::Output;

    /// Return type of [`WindowExpressionMethods::window_order`]
    pub type WindowOrder<Fn, E> = <Fn as OrderWindowDsl<E>>::Output;

    /// Return type of [`WindowExpressionMethods::frame_by`]
    pub type FrameBy<Fn, E> = <Fn as FrameDsl<E>>::Output;

    /// Return type of [`AggregateExpressionMethods::aggregate_distinct`]
    pub type AggregateDistinct<Fn> = <Fn as DistinctDsl>::Output;

    /// Return type of [`AggregateExpressionMethods::aggregate_all`]
    pub type AggregateAll<Fn> = <Fn as AllDsl>::Output;

    /// Return type of [`AggregateExpressionMethods::aggregate_filter`]
    pub type AggregateFilter<Fn, P> = <Fn as FilterDsl<crate::dsl::AsExprOf<P, Bool>>>::Output;

    /// Return type of [`AggregateExpressionMethods::aggregate_order`]
    pub type AggregateOrder<Fn, O> = <Fn as OrderAggregateDsl<O>>::Output;

    /// Return type of [`FrameClauseDsl::frame_start_with`]
    pub type FrameStartWith<S, T> = self::frame_clause::StartFrame<S, T>;

    /// Return type of [`FrameClauseDsl::frame_start_with_exclusion`]
    pub type FrameStartWithExclusion<S, T, E> = self::frame_clause::StartFrame<S, T, E>;

    /// Return type of [`FrameClauseDsl::frame_between`]
    pub type FrameBetween<S, E1, E2> = self::frame_clause::BetweenFrame<S, E1, E2>;

    /// Return type of [`FrameClauseDsl::frame_between_with_exclusion`]
    pub type FrameBetweenWithExclusion<S, E1, E2, E> =
        self::frame_clause::BetweenFrame<S, E1, E2, E>;

    /// Return type of [`FrameBoundDsl::preceding`]
    pub type Preceding<I> = self::frame_clause::OffsetPreceding<I>;

    /// Return type of [`FrameBoundDsl::following`]
    pub type Following<I> = self::frame_clause::OffsetFollowing<I>;
}
