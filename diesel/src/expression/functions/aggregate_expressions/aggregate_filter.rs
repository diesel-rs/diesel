use super::aggregate_order::NoOrder;
use super::prefix::NoPrefix;
use super::AggregateExpression;
use super::IsAggregateFunction;
use super::NoWindow;
use super::NoWithin;
use crate::backend::{sql_dialect, Backend, SqlDialect};
use crate::query_builder::where_clause::NoWhereClause;
use crate::query_builder::where_clause::WhereAnd;
use crate::query_builder::QueryFragment;
use crate::query_builder::{AstPass, QueryId};
use crate::sql_types::BoolOrNullableBool;
use crate::Expression;
use crate::QueryResult;

empty_clause!(NoFilter);

#[derive(QueryId, Copy, Clone, Debug)]
pub struct Filter<P>(P);

impl<P, DB> QueryFragment<DB> for Filter<P>
where
    Self: QueryFragment<DB, DB::AggregateFunctionExpressions>,
    DB: Backend,
{
    fn walk_ast<'b>(&'b self, pass: AstPass<'_, 'b, DB>) -> QueryResult<()> {
        <Self as QueryFragment<DB, DB::AggregateFunctionExpressions>>::walk_ast(self, pass)
    }
}

impl<P, DB>
    QueryFragment<
        DB,
        sql_dialect::aggregate_function_expressions::PostgresLikeAggregateFunctionExpressions,
    > for Filter<P>
where
    P: QueryFragment<DB>,
    DB: Backend + SqlDialect<AggregateFunctionExpressions = sql_dialect::aggregate_function_expressions::PostgresLikeAggregateFunctionExpressions>,
{
    fn walk_ast<'b>(&'b self, mut pass: AstPass<'_, 'b, DB>) -> QueryResult<()> {
        pass.push_sql(" FILTER (");
        self.0.walk_ast(pass.reborrow())?;
        pass.push_sql(")");
        Ok(())
    }
}

pub trait FilterDsl<P> {
    type Output;

    fn filter(self, f: P) -> Self::Output;
}

impl<P, T, ST> FilterDsl<P> for T
where
    T: IsAggregateFunction,
    P: Expression<SqlType = ST>,
    ST: BoolOrNullableBool,
{
    type Output =
        AggregateExpression<T, NoPrefix, NoOrder, Filter<<NoWhereClause as WhereAnd<P>>::Output>>;

    fn filter(self, f: P) -> Self::Output {
        AggregateExpression {
            prefix: NoPrefix,
            function: self,
            order: NoOrder,
            filter: Filter(NoWhereClause.and(f)),
            within_group: NoWithin,
            window: NoWindow,
        }
    }
}

impl<Fn, P, Prefix, Order, F, Within, Window, ST> FilterDsl<P>
    for AggregateExpression<Fn, Prefix, Order, Filter<F>, Within, Window>
where
    P: Expression<SqlType = ST>,
    ST: BoolOrNullableBool,
    F: WhereAnd<P>,
{
    type Output =
        AggregateExpression<Fn, Prefix, Order, Filter<<F as WhereAnd<P>>::Output>, Within, Window>;

    fn filter(self, f: P) -> Self::Output {
        AggregateExpression {
            prefix: self.prefix,
            function: self.function,
            order: self.order,
            filter: Filter(WhereAnd::<P>::and(self.filter.0, f)),
            within_group: self.within_group,
            window: self.window,
        }
    }
}

impl<Fn, P, Prefix, Order, Within, Window, ST> FilterDsl<P>
    for AggregateExpression<Fn, Prefix, Order, NoFilter, Within, Window>
where
    P: Expression<SqlType = ST>,
    ST: BoolOrNullableBool,
    NoWhereClause: WhereAnd<P>,
{
    type Output = AggregateExpression<
        Fn,
        Prefix,
        Order,
        Filter<<NoWhereClause as WhereAnd<P>>::Output>,
        Within,
        Window,
    >;

    fn filter(self, f: P) -> Self::Output {
        AggregateExpression {
            prefix: self.prefix,
            function: self.function,
            order: self.order,
            filter: Filter(WhereAnd::<P>::and(NoWhereClause, f)),
            within_group: self.within_group,
            window: self.window,
        }
    }
}
