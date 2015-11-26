mod dsl_impls;

use expression::*;
use query_source::{QuerySource, Table, LeftOuterJoinSource, InnerJoinSource, JoinTo};
use std::marker::PhantomData;
use super::{Query, QueryBuilder, QueryFragment, BuildQueryResult};
use super::limit_clause::NoLimitClause;
use super::offset_clause::NoOffsetClause;
use super::order_clause::NoOrderClause;
use super::where_clause::NoWhereClause;
use types::{self, NativeSqlType};

#[derive(Debug, Clone, Copy)]
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
            F: Table + JoinTo<T>,
    {
        SelectStatement::new(self.select, self.from.inner_join(other),
            self.where_clause, self.order, self.limit, self.offset)
    }

    pub fn left_outer_join<T>(self, other: T)
        -> SelectStatement<ST, S, LeftOuterJoinSource<F, T>, W, O, L, Of> where
            T: Table,
            F: Table + JoinTo<T>,
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
    SelectStatement<ST, S, F, W, O, L, Of>: QueryFragment
{
    type SqlType = ST;
}

impl<ST, S, F, W, O, L, Of> Expression for SelectStatement<ST, S, F, W, O, L, Of> where
    ST: NativeSqlType,
    F: QuerySource,
    S: SelectableExpression<F, ST>,
    W: QueryFragment,
    O: QueryFragment,
    L: QueryFragment,
    Of: QueryFragment,
{
    type SqlType = types::Array<ST>;

    fn to_sql(&self, out: &mut QueryBuilder) -> BuildQueryResult {
        out.push_sql("SELECT ");
        try!(self.select.to_sql(out));
        out.push_sql(" FROM ");
        try!(self.from.from_clause(out));
        try!(self.where_clause.to_sql(out));
        try!(self.order.to_sql(out));
        try!(self.limit.to_sql(out));
        self.offset.to_sql(out)
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
