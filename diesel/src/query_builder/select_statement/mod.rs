mod dsl_impls;
mod boxed;

pub use self::boxed::BoxedSelectStatement;

use std::marker::PhantomData;

use backend::Backend;
use expression::*;
use query_source::*;
use result::QueryResult;
use super::distinct_clause::NoDistinctClause;
use super::group_by_clause::NoGroupByClause;
use super::limit_clause::NoLimitClause;
use super::offset_clause::NoOffsetClause;
use super::order_clause::NoOrderClause;
use super::where_clause::NoWhereClause;
use super::{Query, QueryBuilder, QueryFragment, BuildQueryResult};

#[derive(Debug, Clone, Copy)]
#[doc(hidden)]
pub struct SelectStatement<
    SqlType,
    Select,
    From,
    Distinct = NoDistinctClause,
    Where = NoWhereClause,
    Order = NoOrderClause,
    Limit = NoLimitClause,
    Offset = NoOffsetClause,
    GroupBy = NoGroupByClause,
> {
    select: Select,
    from: From,
    distinct: Distinct,
    where_clause: Where,
    order: Order,
    limit: Limit,
    offset: Offset,
    group_by: GroupBy,
    _marker: PhantomData<SqlType>,
}

impl<ST, S, F, D, W, O, L, Of, G> SelectStatement<ST, S, F, D, W, O, L, Of, G> {
    pub fn new(
        select: S,
        from: F,
        distinct: D,
        where_clause: W,
        order: O,
        limit: L,
        offset: Of,
        group_by: G,
    ) -> Self {
        SelectStatement {
            select: select,
            from: from,
            distinct: distinct,
            where_clause: where_clause,
            order: order,
            limit: limit,
            offset: offset,
            group_by: group_by,
            _marker: PhantomData,
        }
    }

    pub fn inner_join<T>(self, other: T)
        -> SelectStatement<ST, S, InnerJoinSource<F, T>, D, W, O, L, Of, G> where
            T: Table,
            F: Table + JoinTo<T, joins::Inner>,
    {
        SelectStatement::new(
            self.select,
            self.from.inner_join(other),
            self.distinct,
            self.where_clause,
            self.order,
            self.limit,
            self.offset,
            self.group_by,
        )
    }

    pub fn left_outer_join<T>(self, other: T)
        -> SelectStatement<ST, S, LeftOuterJoinSource<F, T>, D, W, O, L, Of, G> where
            T: Table,
            F: Table + JoinTo<T, joins::LeftOuter>,
    {
        SelectStatement::new(
            self.select,
            self.from.left_outer_join(other),
            self.distinct,
            self.where_clause,
            self.order,
            self.limit,
            self.offset,
            self.group_by,
        )
    }
}

impl<ST, S, F> SelectStatement<ST, S, F> {
    pub fn simple(select: S, from: F) -> Self {
        SelectStatement::new(
            select,
            from,
            NoDistinctClause,
            NoWhereClause,
            NoOrderClause,
            NoLimitClause,
            NoOffsetClause,
            NoGroupByClause,
        )
    }
}

impl<ST, S, F, D, W, O, L, Of, G> Query
    for SelectStatement<ST, S, F, D, W, O, L, Of, G> where
        S: SelectableExpression<F, ST>,
{
    type SqlType = ST;
}

#[cfg(feature = "postgres")]
impl<ST, S, F, D, W, O, L, Of, G> Expression
    for SelectStatement<ST, S, F, D, W, O, L, Of, G> where
        S: SelectableExpression<F, ST>,
{
    type SqlType = ::types::Array<ST>;
}

#[cfg(not(feature = "postgres"))]
impl<ST, S, F, D, W, O, L, Of, G> Expression
    for SelectStatement<ST, S, F, D, W, O, L, Of, G> where
        S: SelectableExpression<F, ST>,
{
    type SqlType = ST;
}

impl<ST, S, F, D, W, O, L, Of, G, DB> QueryFragment<DB>
    for SelectStatement<ST, S, F, D, W, O, L, Of, G> where
        DB: Backend,
        S: QueryFragment<DB>,
        F: QuerySource,
        F::FromClause: QueryFragment<DB>,
        D: QueryFragment<DB>,
        W: QueryFragment<DB>,
        O: QueryFragment<DB>,
        L: QueryFragment<DB>,
        Of: QueryFragment<DB>,
        G: QueryFragment<DB>,
{
    fn to_sql(&self, out: &mut DB::QueryBuilder) -> BuildQueryResult {
        out.push_sql("SELECT ");
        try!(self.distinct.to_sql(out));
        try!(self.select.to_sql(out));
        out.push_sql(" FROM ");
        try!(self.from.from_clause().to_sql(out));
        try!(self.where_clause.to_sql(out));
        try!(self.group_by.to_sql(out));
        try!(self.order.to_sql(out));
        try!(self.limit.to_sql(out));
        try!(self.offset.to_sql(out));
        Ok(())
    }

    fn collect_binds(&self, out: &mut DB::BindCollector) -> QueryResult<()> {
        try!(self.distinct.collect_binds(out));
        try!(self.select.collect_binds(out));
        try!(self.from.from_clause().collect_binds(out));
        try!(self.where_clause.collect_binds(out));
        try!(self.group_by.collect_binds(out));
        try!(self.order.collect_binds(out));
        try!(self.limit.collect_binds(out));
        try!(self.offset.collect_binds(out));
        Ok(())
    }

    fn is_safe_to_cache_prepared(&self) -> bool {
        self.distinct.is_safe_to_cache_prepared() &&
            self.select.is_safe_to_cache_prepared() &&
            self.from.from_clause().is_safe_to_cache_prepared() &&
            self.where_clause.is_safe_to_cache_prepared() &&
            self.group_by.is_safe_to_cache_prepared() &&
            self.order.is_safe_to_cache_prepared() &&
            self.limit.is_safe_to_cache_prepared() &&
            self.offset.is_safe_to_cache_prepared()
    }
}

impl<ST, S, D, W, O, L, Of, G, DB> QueryFragment<DB>
    for SelectStatement<ST, S, (), D, W, O, L, Of, G> where
        DB: Backend,
        S: QueryFragment<DB>,
        D: QueryFragment<DB>,
        W: QueryFragment<DB>,
        O: QueryFragment<DB>,
        L: QueryFragment<DB>,
        Of: QueryFragment<DB>,
        G: QueryFragment<DB>,
{
    fn to_sql(&self, out: &mut DB::QueryBuilder) -> BuildQueryResult {
        out.push_sql("SELECT ");
        try!(self.distinct.to_sql(out));
        try!(self.select.to_sql(out));
        try!(self.where_clause.to_sql(out));
        try!(self.group_by.to_sql(out));
        try!(self.order.to_sql(out));
        try!(self.limit.to_sql(out));
        try!(self.offset.to_sql(out));
        Ok(())
    }

    fn collect_binds(&self, out: &mut DB::BindCollector) -> QueryResult<()> {
        try!(self.distinct.collect_binds(out));
        try!(self.select.collect_binds(out));
        try!(self.where_clause.collect_binds(out));
        try!(self.group_by.collect_binds(out));
        try!(self.order.collect_binds(out));
        try!(self.limit.collect_binds(out));
        try!(self.offset.collect_binds(out));
        Ok(())
    }

    fn is_safe_to_cache_prepared(&self) -> bool {
        self.distinct.is_safe_to_cache_prepared() &&
            self.select.is_safe_to_cache_prepared() &&
            self.where_clause.is_safe_to_cache_prepared() &&
            self.group_by.is_safe_to_cache_prepared() &&
            self.order.is_safe_to_cache_prepared() &&
            self.limit.is_safe_to_cache_prepared() &&
            self.offset.is_safe_to_cache_prepared()
    }
}

impl_query_id!(SelectStatement<ST, S, F, D, W, O, L, Of, G>);

impl<ST, S, F, D, W, O, L, Of, G, QS> SelectableExpression<QS>
    for SelectStatement<ST, S, F, D, W, O, L, Of, G> where
        SelectStatement<ST, S, F, D, W, O, L, Of, G>: Expression,
{
}

impl<ST, S, F, D, W, O, L, Of, G> NonAggregate
    for SelectStatement<ST, S, F, D, W, O, L, Of, G> where
        SelectStatement<ST, S, F, D, W, O, L, Of, G>: Expression,
{
}
