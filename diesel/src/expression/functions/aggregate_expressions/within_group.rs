use super::AggregateExpression;
use super::All;
use super::IsAggregateFunction;
use super::NoFilter;
use super::NoOrder;
use super::NoPrefix;
use super::NoWindow;
use crate::query_builder::order_clause::OrderClause;
use crate::query_builder::QueryFragment;
use crate::query_builder::{AstPass, QueryId};
use crate::Expression;
use crate::QueryResult;

empty_clause!(NoWithin);

#[derive(QueryId, Copy, Clone, Debug)]
pub struct WithinGroup<T>(OrderClause<T>);

// this clause is only postgres specific
#[cfg(feature = "postgres_backend")]
impl<E> QueryFragment<diesel::pg::Pg> for WithinGroup<E>
where
    OrderClause<E>: QueryFragment<diesel::pg::Pg>,
{
    fn walk_ast<'b>(&'b self, mut pass: AstPass<'_, 'b, diesel::pg::Pg>) -> QueryResult<()> {
        pass.push_sql(" WITHIN GROUP (");
        self.0.walk_ast(pass.reborrow())?;
        pass.push_sql(")");
        Ok(())
    }
}

pub trait WithinGroupDsl<E> {
    type Output;

    fn within_group(self, expr: E) -> Self::Output;
}

impl<E, T> WithinGroupDsl<E> for T
where
    T: IsAggregateFunction,
    E: Expression,
{
    type Output = AggregateExpression<T, NoPrefix, NoOrder, NoFilter, WithinGroup<E>>;

    fn within_group(self, expr: E) -> Self::Output {
        AggregateExpression {
            prefix: NoPrefix,
            function: self,
            order: NoOrder,
            filter: NoFilter,
            within_group: WithinGroup(OrderClause(expr)),
            window: NoWindow,
        }
    }
}

impl<O, Fn, Filter, Within> WithinGroupDsl<O>
    for AggregateExpression<Fn, NoPrefix, NoOrder, Filter, Within, NoWindow>
{
    type Output = AggregateExpression<Fn, NoPrefix, NoOrder, Filter, WithinGroup<O>>;

    fn within_group(self, expr: O) -> Self::Output {
        AggregateExpression {
            prefix: self.prefix,
            function: self.function,
            order: self.order,
            filter: self.filter,
            within_group: WithinGroup(OrderClause(expr)),
            window: NoWindow,
        }
    }
}

impl<O, Fn, Filter, Within> WithinGroupDsl<O>
    for AggregateExpression<Fn, All, NoOrder, Filter, Within, NoWindow>
{
    type Output = AggregateExpression<Fn, All, NoOrder, Filter, WithinGroup<O>>;

    fn within_group(self, expr: O) -> Self::Output {
        AggregateExpression {
            prefix: self.prefix,
            function: self.function,
            order: self.order,
            filter: self.filter,
            within_group: WithinGroup(OrderClause(expr)),
            window: NoWindow,
        }
    }
}
