use super::aggregate_filter::{Filter, NoFilter};
use super::aggregate_order::NoOrder;
use super::partition_by::NoPartition;
use super::prefix::NoPrefix;
use super::IsWindowFunction;
use super::NoFrame;
use super::WindowFunctionFragment;
use super::{AggregateExpression, IsAggregateFunction};
use crate::query_builder::QueryFragment;
use crate::query_builder::{AstPass, QueryId};
use crate::QueryResult;

/// Only aggregate functions allow to use filter
pub(super) trait ValidAggregateFilterForWindow<Fn, Window> {}

impl<Fn, W> ValidAggregateFilterForWindow<Fn, W> for NoFilter {}
empty_clause!(NoWindow);

impl<F, Fn> ValidAggregateFilterForWindow<Fn, NoWindow> for Filter<F> {}

impl<DB, T> WindowFunctionFragment<T, DB> for NoWindow where DB: crate::backend::Backend {}

#[derive(Clone, Copy, QueryId, Debug)]
#[doc(hidden)] // not even sure why rustc believes this is public
#[diesel(diesel_internal_is_window = true)]
pub struct OverClause<Partition = NoPartition, Order = NoOrder, Frame = NoFrame> {
    pub(crate) partition_by: Partition,
    pub(crate) order: Order,
    pub(crate) frame_clause: Frame,
}

impl<F, Fn, Partition, Order, Frame>
    ValidAggregateFilterForWindow<Fn, OverClause<Partition, Order, Frame>> for Filter<F>
where
    Fn: IsAggregateFunction,
{
}

impl<Partition, Order, Frame, DB> QueryFragment<DB> for OverClause<Partition, Order, Frame>
where
    Partition: QueryFragment<DB>,
    Order: QueryFragment<DB>,
    Frame: QueryFragment<DB>,
    DB: crate::backend::Backend,
{
    fn walk_ast<'b>(&'b self, mut pass: AstPass<'_, 'b, DB>) -> QueryResult<()> {
        pass.push_sql(" OVER (");
        self.partition_by.walk_ast(pass.reborrow())?;
        self.order.walk_ast(pass.reborrow())?;
        self.frame_clause.walk_ast(pass.reborrow())?;
        pass.push_sql(")");
        Ok(())
    }
}

pub trait OverDsl {
    type Output;

    fn over(self) -> Self::Output;
}

impl<F> OverDsl for F
where
    F: IsWindowFunction,
{
    type Output = AggregateExpression<F, NoPrefix, NoOrder, NoFilter, OverClause>;

    fn over(self) -> Self::Output {
        AggregateExpression {
            prefix: NoPrefix,
            function: self,
            order: NoOrder,
            filter: NoFilter,
            window: OverClause {
                partition_by: NoPartition,
                order: NoOrder,
                frame_clause: NoFrame,
            },
        }
    }
}

impl<Fn, Filter> OverDsl for AggregateExpression<Fn, NoPrefix, NoOrder, Filter, NoWindow>
where
    Filter: ValidAggregateFilterForWindow<Fn, OverClause>,
{
    type Output = AggregateExpression<Fn, NoPrefix, NoOrder, Filter, OverClause>;

    fn over(self) -> Self::Output {
        AggregateExpression {
            prefix: NoPrefix,
            function: self.function,
            order: NoOrder,
            filter: self.filter,
            window: OverClause {
                partition_by: NoPartition,
                order: NoOrder,
                frame_clause: NoFrame,
            },
        }
    }
}
