use super::aggregate_filter::NoFilter;
use super::aggregate_order::NoOrder;
use super::frame_clause::NoFrame;
use super::over_clause::{OverClause, ValidAggregateFilterForWindow};
use super::prefix::NoPrefix;
use super::NoWindow;
use super::{AggregateExpression, IsWindowFunction};
use crate::query_builder::QueryFragment;
use crate::query_builder::{AstPass, QueryId};
use crate::QueryResult;

empty_clause!(NoPartition);

#[derive(QueryId, Clone, Copy, Debug)]
pub struct PartitionBy<T>(T);

impl<T, DB> QueryFragment<DB> for PartitionBy<T>
where
    T: QueryFragment<DB>,
    DB: crate::backend::Backend,
{
    fn walk_ast<'b>(&'b self, mut pass: AstPass<'_, 'b, DB>) -> QueryResult<()> {
        pass.push_sql(" PARTITION BY ");
        self.0.walk_ast(pass.reborrow())?;
        Ok(())
    }
}

pub trait PartitionByDsl<E> {
    type Output;

    fn partition_by(self, expr: E) -> Self::Output;
}

impl<E, Fn, Filter, Frame, Partition, Order> PartitionByDsl<E>
    for AggregateExpression<Fn, NoPrefix, NoOrder, Filter, OverClause<Partition, Order, Frame>>
where
    Filter: ValidAggregateFilterForWindow<Fn, OverClause<PartitionBy<E>, Order, Frame>>,
{
    type Output = AggregateExpression<
        Fn,
        NoPrefix,
        NoOrder,
        Filter,
        OverClause<PartitionBy<E>, Order, Frame>,
    >;

    fn partition_by(self, expr: E) -> Self::Output {
        AggregateExpression {
            prefix: NoPrefix,
            function: self.function,
            order: NoOrder,
            filter: self.filter,
            window: OverClause {
                partition_by: PartitionBy(expr),
                order: self.window.order,
                frame_clause: self.window.frame_clause,
            },
        }
    }
}

impl<E, Fn, Filter> PartitionByDsl<E>
    for AggregateExpression<Fn, NoPrefix, NoOrder, Filter, NoWindow>
where
    Filter: ValidAggregateFilterForWindow<Fn, OverClause<PartitionBy<E>, NoOrder, NoFrame>>,
{
    type Output = AggregateExpression<
        Fn,
        NoPrefix,
        NoOrder,
        Filter,
        OverClause<PartitionBy<E>, NoOrder, NoFrame>,
    >;

    fn partition_by(self, expr: E) -> Self::Output {
        AggregateExpression {
            prefix: NoPrefix,
            function: self.function,
            order: NoOrder,
            filter: self.filter,
            window: OverClause {
                partition_by: PartitionBy(expr),
                order: NoOrder,
                frame_clause: NoFrame,
            },
        }
    }
}

impl<E, Fn> PartitionByDsl<E> for Fn
where
    Fn: IsWindowFunction,
{
    type Output = AggregateExpression<
        Fn,
        NoPrefix,
        NoOrder,
        NoFilter,
        OverClause<PartitionBy<E>, NoOrder, NoFrame>,
    >;

    fn partition_by(self, expr: E) -> Self::Output {
        AggregateExpression {
            prefix: NoPrefix,
            function: self,
            order: NoOrder,
            filter: NoFilter,
            window: OverClause {
                partition_by: PartitionBy(expr),
                order: NoOrder,
                frame_clause: NoFrame,
            },
        }
    }
}
