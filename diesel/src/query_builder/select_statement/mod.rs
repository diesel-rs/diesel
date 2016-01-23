mod dsl_impls;

use backend::Backend;
use expression::*;
use query_source::*;
use std::marker::PhantomData;
use super::{Query, QueryBuilder, QueryFragment, BuildQueryResult, Context};
use super::limit_clause::NoLimitClause;
use super::offset_clause::NoOffsetClause;
use super::order_clause::NoOrderClause;
use super::where_clause::NoWhereClause;
use types::{self, NativeSqlType};

#[derive(Debug, Clone, Copy)]
#[doc(hidden)]
pub struct SelectStatement<
    SqlType,
    Select,
    From,
    Where = NoWhereClause,
    Order = NoOrderClause,
    Limit = NoLimitClause,
    Offset = NoOffsetClause,
> {
    select: Select,
    from: From,
    where_clause: Where,
    order: Order,
    limit: Limit,
    offset: Offset,
    _marker: PhantomData<SqlType>,
}

impl<ST, S, F, W, O, L, Of> SelectStatement<ST, S, F, W, O, L, Of> {
    pub fn new(select: S, from: F, where_clause: W, order: O, limit: L, offset: Of) -> Self {
        SelectStatement {
            select: select,
            from: from,
            where_clause: where_clause,
            order: order,
            limit: limit,
            offset: offset,
            _marker: PhantomData,
        }
    }

    pub fn inner_join<T>(self, other: T)
        -> SelectStatement<ST, S, InnerJoinSource<F, T>, W, O, L, Of> where
            T: Table,
            F: Table + JoinTo<T, joins::Inner>,
    {
        SelectStatement::new(self.select, self.from.inner_join(other),
            self.where_clause, self.order, self.limit, self.offset)
    }

    pub fn left_outer_join<T>(self, other: T)
        -> SelectStatement<ST, S, LeftOuterJoinSource<F, T>, W, O, L, Of> where
            T: Table,
            F: Table + JoinTo<T, joins::LeftOuter>,
    {
        SelectStatement::new(self.select, self.from.left_outer_join(other),
            self.where_clause, self.order, self.limit, self.offset)
    }
}

impl<ST, S, F> SelectStatement<ST, S, F> {
    pub fn simple(select: S, from: F) -> Self {
        SelectStatement::new(select, from, NoWhereClause, NoOrderClause, NoLimitClause, NoOffsetClause)
    }
}

impl<ST, S, F, W, O, L, Of> Query for SelectStatement<ST, S, F, W, O, L, Of> where
    ST: NativeSqlType,
    S: SelectableExpression<F, ST>,
{
    type SqlType = ST;
}

impl<ST, S, F, W, O, L, Of> Expression for SelectStatement<ST, S, F, W, O, L, Of> where
    ST: NativeSqlType,
    S: SelectableExpression<F, ST>,
{
    type SqlType = types::Array<ST>;
}

impl<ST, S, F, W, O, L, Of, DB> QueryFragment<DB> for SelectStatement<ST, S, F, W, O, L, Of> where
    DB: Backend,
    S: QueryFragment<DB>,
    F: QuerySource,
    F::FromClause: QueryFragment<DB>,
    W: QueryFragment<DB>,
    O: QueryFragment<DB>,
    L: QueryFragment<DB>,
    Of: QueryFragment<DB>,
{
    fn to_sql(&self, out: &mut DB::QueryBuilder) -> BuildQueryResult {
        out.push_context(Context::Select);
        out.push_sql("SELECT ");
        try!(self.select.to_sql(out));
        out.push_sql(" FROM ");
        try!(self.from.from_clause().to_sql(out));
        try!(self.where_clause.to_sql(out));
        try!(self.order.to_sql(out));
        try!(self.limit.to_sql(out));
        try!(self.offset.to_sql(out));
        out.pop_context();
        Ok(())
    }
}

impl<ST, S, W, O, L, Of, DB> QueryFragment<DB> for SelectStatement<ST, S, (), W, O, L, Of> where
    DB: Backend,
    S: QueryFragment<DB>,
    W: QueryFragment<DB>,
    O: QueryFragment<DB>,
    L: QueryFragment<DB>,
    Of: QueryFragment<DB>,
{
    fn to_sql(&self, out: &mut DB::QueryBuilder) -> BuildQueryResult {
        out.push_context(Context::Select);
        out.push_sql("SELECT ");
        try!(self.select.to_sql(out));
        try!(self.where_clause.to_sql(out));
        try!(self.order.to_sql(out));
        try!(self.limit.to_sql(out));
        try!(self.offset.to_sql(out));
        out.pop_context();
        Ok(())
    }
}

impl<ST, S, F, W, O, L, Of, QS> SelectableExpression<QS> for SelectStatement<ST, S, F, W, O, L, Of> where
    SelectStatement<ST, S, F, W, O, L, Of>: Expression,
{
}

impl<ST, S, F, W, O, L, Of> NonAggregate for SelectStatement<ST, S, F, W, O, L, Of> where
    SelectStatement<ST, S, F, W, O, L, Of>: Expression,
{
}
