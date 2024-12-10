use super::AggregateExpression;
use super::IsAggregateFunction;
use super::NoFilter;
use super::NoOrder;
use super::NoWindow;
use super::NoWithin;
use crate::query_builder::{AstPass, QueryFragment, QueryId};
use crate::QueryResult;

empty_clause!(NoPrefix);

#[derive(Debug, Clone, Copy, QueryId)]
pub struct All;

impl<DB> QueryFragment<DB> for All
where
    DB: crate::backend::Backend,
{
    fn walk_ast<'b>(&'b self, mut pass: AstPass<'_, 'b, DB>) -> QueryResult<()> {
        pass.push_sql(" ALL ");
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, QueryId)]
pub struct Distinct;

impl<DB> QueryFragment<DB> for Distinct
where
    DB: crate::backend::Backend,
{
    fn walk_ast<'b>(&'b self, mut pass: AstPass<'_, 'b, DB>) -> QueryResult<()> {
        pass.push_sql(" DISTINCT ");
        Ok(())
    }
}

pub trait DistinctDsl {
    type Output;

    fn distinct(self) -> Self::Output;
}

impl<T> DistinctDsl for T
where
    T: IsAggregateFunction,
{
    type Output = AggregateExpression<T, Distinct>;

    fn distinct(self) -> Self::Output {
        AggregateExpression {
            prefix: Distinct,
            function: self,
            order: NoOrder,
            filter: NoFilter,
            within_group: NoWithin,
            window: NoWindow,
        }
    }
}

impl<T, Prefix, Order, Filter, Within> DistinctDsl
    for AggregateExpression<T, Prefix, Order, Filter, Within, NoWindow>
where
    T: IsAggregateFunction,
{
    type Output = AggregateExpression<T, Distinct, Order, Filter, Within, NoWindow>;

    fn distinct(self) -> Self::Output {
        AggregateExpression {
            prefix: Distinct,
            function: self.function,
            order: self.order,
            filter: self.filter,
            within_group: self.within_group,
            window: self.window,
        }
    }
}

pub trait AllDsl {
    type Output;

    fn all(self) -> Self::Output;
}

impl<T> AllDsl for T
where
    T: IsAggregateFunction,
{
    type Output = AggregateExpression<T, All>;

    fn all(self) -> Self::Output {
        AggregateExpression {
            prefix: All,
            function: self,
            order: NoOrder,
            filter: NoFilter,
            within_group: NoWithin,
            window: NoWindow,
        }
    }
}

impl<T, Prefix, Order, Filter, Within> AllDsl
    for AggregateExpression<T, Prefix, Order, Filter, Within, NoWindow>
where
    T: IsAggregateFunction,
{
    type Output = AggregateExpression<T, All, Order, Filter, Within, NoWindow>;

    fn all(self) -> Self::Output {
        AggregateExpression {
            prefix: All,
            function: self.function,
            order: self.order,
            filter: self.filter,
            within_group: self.within_group,
            window: self.window,
        }
    }
}
