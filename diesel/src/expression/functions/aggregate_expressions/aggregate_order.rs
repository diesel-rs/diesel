use super::IsAggregateFunction;
use super::NoFilter;
use super::NoPrefix;
use super::NoWindow;
use super::NoWithin;
use super::{over_clause::OverClause, AggregateExpression};
use crate::backend::{sql_dialect, Backend, SqlDialect};
use crate::query_builder::order_clause::OrderClause;
use crate::query_builder::QueryFragment;
use crate::query_builder::{AstPass, QueryId};
use crate::{Expression, QueryResult};

empty_clause!(NoOrder);

#[derive(QueryId, Copy, Clone, Debug)]
pub struct Order<T>(OrderClause<T>);

impl<E, DB> QueryFragment<DB> for Order<E>
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
    > for Order<E>
where
    OrderClause<E>: QueryFragment<DB>,
    DB: Backend +SqlDialect<AggregateFunctionExpressions = sql_dialect::aggregate_function_expressions::PostgresLikeAggregateFunctionExpressions>,
{
    fn walk_ast<'b>(&'b self, mut pass: AstPass<'_, 'b, DB>) -> QueryResult<()> {
        self.0.walk_ast(pass.reborrow())?;
        Ok(())
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
    type Output = AggregateExpression<T, NoPrefix, Order<E>>;

    fn order(self, expr: E) -> Self::Output {
        AggregateExpression {
            prefix: NoPrefix,
            function: self,
            order: Order(OrderClause(expr)),
            filter: NoFilter,
            within_group: NoWithin,
            window: NoWindow,
        }
    }
}

impl<O, Fn, Prefix, Ord, Filter> OrderAggregateDsl<O>
    for AggregateExpression<Fn, Prefix, Ord, Filter, NoWithin, NoWindow>
{
    type Output = AggregateExpression<Fn, Prefix, Order<O>, Filter>;

    fn order(self, expr: O) -> Self::Output {
        AggregateExpression {
            prefix: self.prefix,
            function: self.function,
            order: Order(OrderClause(expr)),
            filter: self.filter,
            within_group: self.within_group,
            window: NoWindow,
        }
    }
}

pub trait OrderWindowDsl<O> {
    type Output;

    fn order(self, expr: O) -> Self::Output;
}

impl<E, Fn, Filter, Frame, Partition, O> OrderWindowDsl<E>
    for AggregateExpression<
        Fn,
        NoPrefix,
        NoOrder,
        Filter,
        NoWithin,
        OverClause<Partition, O, Frame>,
    >
{
    type Output = AggregateExpression<
        Fn,
        NoPrefix,
        NoOrder,
        Filter,
        NoWithin,
        OverClause<Partition, Order<E>, Frame>,
    >;

    fn order(self, expr: E) -> Self::Output {
        AggregateExpression {
            prefix: NoPrefix,
            function: self.function,
            order: NoOrder,
            filter: self.filter,
            within_group: NoWithin,
            window: OverClause {
                partition_by: self.window.partition_by,
                order: Order(OrderClause(expr)),
                frame_clause: self.window.frame_clause,
            },
        }
    }
}
