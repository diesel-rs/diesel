use super::aggregate_filter::NoFilter;
use super::aggregate_order::NoOrder;
use super::partition_by::NoPartition;
use super::prefix::NoPrefix;
use super::within_group::NoWithin;
use super::AggregateExpression;
use super::IsWindowFunction;
use super::NoFrame;
use super::WindowFunctionFragment;
use crate::query_builder::QueryFragment;
use crate::query_builder::{AstPass, QueryId};
use crate::QueryResult;

empty_clause!(NoWindow);

impl<DB, T> WindowFunctionFragment<T, DB> for NoWindow where DB: crate::backend::Backend {}

/// TODO
#[derive(Clone, Copy, QueryId, Debug)]
pub struct OverClause<Partition = NoPartition, Order = NoOrder, Frame = NoFrame> {
    pub(crate) partition_by: Partition,
    pub(crate) order: Order,
    pub(crate) frame_clause: Frame,
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
    type Output = AggregateExpression<F, NoPrefix, NoOrder, NoFilter, NoWithin, OverClause>;

    fn over(self) -> Self::Output {
        AggregateExpression {
            prefix: NoPrefix,
            function: self,
            order: NoOrder,
            filter: NoFilter,
            within_group: NoWithin,
            window: OverClause {
                partition_by: NoPartition,
                order: NoOrder,
                frame_clause: NoFrame,
            },
        }
    }
}

impl<Fn, Filter> OverDsl
    for AggregateExpression<Fn, NoPrefix, NoOrder, Filter, NoWithin, NoWindow>
{
    type Output = AggregateExpression<Fn, NoPrefix, NoOrder, Filter, NoWithin, OverClause>;

    fn over(self) -> Self::Output {
        AggregateExpression {
            prefix: NoPrefix,
            function: self.function,
            order: NoOrder,
            filter: self.filter,
            within_group: NoWithin,
            window: OverClause {
                partition_by: NoPartition,
                order: NoOrder,
                frame_clause: NoFrame,
            },
        }
    }
}
