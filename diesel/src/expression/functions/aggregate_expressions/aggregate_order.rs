use super::frame_clause::NoFrame;
use super::over_clause::ValidAggregateFilterForWindow;
use super::partition_by::NoPartition;
use super::NoFilter;
use super::NoPrefix;
use super::NoWindow;
use super::{over_clause::OverClause, AggregateExpression};
use super::{IsAggregateFunction, IsWindowFunction};
use crate::backend::{sql_dialect, Backend, SqlDialect};
use crate::query_builder::order_clause::OrderClause;
use crate::query_builder::QueryFragment;
use crate::query_builder::{AstPass, QueryId};
use crate::{Expression, QueryResult};

empty_clause!(NoOrder);

/// A order clause for window and aggregate function expressions
#[derive(QueryId, Copy, Clone, Debug)]
pub struct Order<T, const WINDOW: bool>(OrderClause<T>);

impl<E, DB> QueryFragment<DB> for Order<E, false>
where
    Self: QueryFragment<DB, DB::AggregateFunctionExpressions>,
    DB: Backend,
{
    fn walk_ast<'b>(&'b self, pass: AstPass<'_, 'b, DB>) -> QueryResult<()> {
        <Self as QueryFragment<DB, DB::AggregateFunctionExpressions>>::walk_ast(self, pass)
    }
}

impl<E, DB>
    QueryFragment<
        DB,
        sql_dialect::aggregate_function_expressions::PostgresLikeAggregateFunctionExpressions,
    > for Order<E, false>
where
    OrderClause<E>: QueryFragment<DB>,
    DB: Backend + SqlDialect<
        AggregateFunctionExpressions = sql_dialect::aggregate_function_expressions::PostgresLikeAggregateFunctionExpressions
    >,
{
    fn walk_ast<'b>(&'b self, mut pass: AstPass<'_, 'b, DB>) -> QueryResult<()> {
        self.0.walk_ast(pass.reborrow())?;
        Ok(())
    }
}

impl<E, DB> QueryFragment<DB> for Order<E, true>
where
    OrderClause<E>: QueryFragment<DB>,
    DB: Backend,
{
    fn walk_ast<'b>(&'b self, pass: AstPass<'_, 'b, DB>) -> QueryResult<()> {
        self.0.walk_ast(pass)
    }
}

pub trait OrderAggregateDsl<E> {
    type Output;

    fn order(self, expr: E) -> Self::Output;
}

impl<E, T> OrderAggregateDsl<E> for T
where
    T: IsAggregateFunction,
    E: Expression,
{
    type Output = AggregateExpression<T, NoPrefix, Order<E, false>>;

    fn order(self, expr: E) -> Self::Output {
        AggregateExpression {
            prefix: NoPrefix,
            function: self,
            order: Order(OrderClause(expr)),
            filter: NoFilter,
            window: NoWindow,
        }
    }
}

impl<O, Fn, Prefix, Ord, Filter> OrderAggregateDsl<O>
    for AggregateExpression<Fn, Prefix, Ord, Filter, NoWindow>
{
    type Output = AggregateExpression<Fn, Prefix, Order<O, false>, Filter>;

    fn order(self, expr: O) -> Self::Output {
        AggregateExpression {
            prefix: self.prefix,
            function: self.function,
            order: Order(OrderClause(expr)),
            filter: self.filter,
            window: NoWindow,
        }
    }
}

pub trait OrderWindowDsl<O> {
    type Output;

    fn order(self, expr: O) -> Self::Output;
}

impl<E, Fn, Filter, Frame, Partition, O> OrderWindowDsl<E>
    for AggregateExpression<Fn, NoPrefix, NoOrder, Filter, OverClause<Partition, O, Frame>>
where
    Filter: ValidAggregateFilterForWindow<Fn, OverClause<Partition, Order<E, true>, Frame>>,
{
    type Output = AggregateExpression<
        Fn,
        NoPrefix,
        NoOrder,
        Filter,
        OverClause<Partition, Order<E, true>, Frame>,
    >;

    fn order(self, expr: E) -> Self::Output {
        AggregateExpression {
            prefix: NoPrefix,
            function: self.function,
            order: NoOrder,
            filter: self.filter,
            window: OverClause {
                partition_by: self.window.partition_by,
                order: Order(OrderClause(expr)),
                frame_clause: self.window.frame_clause,
            },
        }
    }
}

impl<Fn, Filter, E> OrderWindowDsl<E>
    for AggregateExpression<Fn, NoPrefix, NoOrder, Filter, NoWindow>
where
    Filter: ValidAggregateFilterForWindow<Fn, OverClause<NoPartition, Order<E, true>, NoFrame>>,
{
    type Output = AggregateExpression<
        Fn,
        NoPrefix,
        NoOrder,
        Filter,
        OverClause<NoPartition, Order<E, true>, NoFrame>,
    >;

    fn order(self, expr: E) -> Self::Output {
        AggregateExpression {
            prefix: NoPrefix,
            function: self.function,
            order: NoOrder,
            filter: self.filter,
            window: OverClause {
                partition_by: NoPartition,
                order: Order(OrderClause(expr)),
                frame_clause: NoFrame,
            },
        }
    }
}

impl<O, Fn> OrderWindowDsl<O> for Fn
where
    Fn: IsWindowFunction,
{
    type Output = AggregateExpression<
        Fn,
        NoPrefix,
        NoOrder,
        NoFilter,
        OverClause<NoPartition, Order<O, true>, NoFrame>,
    >;

    fn order(self, expr: O) -> Self::Output {
        AggregateExpression {
            prefix: NoPrefix,
            function: self,
            order: NoOrder,
            filter: NoFilter,
            window: OverClause {
                partition_by: NoPartition,
                order: Order(OrderClause(expr)),
                frame_clause: NoFrame,
            },
        }
    }
}
