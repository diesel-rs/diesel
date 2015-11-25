mod dsl_impls;

use expression::*;
use query_source::QuerySource;
use std::marker::PhantomData;
use super::{Query, QueryBuilder, QueryFragment, BuildQueryResult};
use super::where_clause::NoWhereClause;
use super::order_clause::NoOrderClause;
use super::limit_clause::NoLimitClause;
use types::{self, NativeSqlType};

#[derive(Debug, Clone, Copy)]
pub struct SelectStatement<
    SqlType,
    Select,
    From,
    Where = NoWhereClause,
    Order = NoOrderClause,
    Limit = NoLimitClause,
> {
    select: Select,
    from: From,
    where_clause: Where,
    order: Order,
    limit: Limit,
    _marker: PhantomData<SqlType>,
}

impl<ST, S, F, W, O, L> SelectStatement<ST, S, F, W, O, L> {
    pub fn new(select: S, from: F, where_clause: W, order: O, limit: L) -> Self {
        SelectStatement {
            select: select,
            from: from,
            where_clause: where_clause,
            order: order,
            limit: limit,
            _marker: PhantomData,
        }
    }
}

impl<ST, S, F> SelectStatement<ST, S, F> {
    pub fn simple(select: S, from: F) -> Self {
        SelectStatement::new(select, from, NoWhereClause, NoOrderClause, NoLimitClause)
    }
}

impl<ST, S, F, W, O, L> Query for SelectStatement<ST, S, F, W, O, L> where
    ST: NativeSqlType,
    SelectStatement<ST, S, F, W, O, L>: QueryFragment
{
    type SqlType = ST;
}

impl<ST, S, F, W, O, L> Expression for SelectStatement<ST, S, F, W, O, L> where
    ST: NativeSqlType,
    F: QuerySource,
    S: SelectableExpression<F, ST>,
    W: QueryFragment,
    O: QueryFragment,
    L: QueryFragment,
{
    type SqlType = types::Array<ST>;

    fn to_sql(&self, out: &mut QueryBuilder) -> BuildQueryResult {
        out.push_sql("SELECT ");
        try!(self.select.to_sql(out));
        out.push_sql(" FROM ");
        try!(self.from.from_clause(out));
        try!(self.where_clause.to_sql(out));
        try!(self.order.to_sql(out));
        self.limit.to_sql(out)
    }
}

impl<ST, S, F, W, O, L, QS> SelectableExpression<QS> for SelectStatement<ST, S, F, W, O, L> where
    SelectStatement<ST, S, F, W, O, L>: Expression,
{
}

impl<ST, S, F, W, O, L> NonAggregate for SelectStatement<ST, S, F, W, O, L> where
    SelectStatement<ST, S, F, W, O, L>: Expression,
{
}
