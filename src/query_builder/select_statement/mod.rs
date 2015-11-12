mod dsl_impls;

use expression::*;
use query_source::QuerySource;
use std::marker::PhantomData;
use super::{Query, QueryBuilder, QueryFragment, BuildQueryResult};
use super::where_clause::NoWhereClause;
use super::order_clause::NoOrderClause;
use types::NativeSqlType;

#[derive(Debug, Clone, Copy)]
pub struct SelectStatement<
    SqlType,
    Select,
    From,
    Where = NoWhereClause,
    Order = NoOrderClause,
> {
    select: Select,
    from: From,
    where_clause: Where,
    order: Order,
    _marker: PhantomData<SqlType>,
}

impl<ST, S, F, W, O> SelectStatement<ST, S, F, W, O> {
    pub fn new(select: S, from: F, where_clause: W, order: O) -> Self {
        SelectStatement {
            select: select,
            from: from,
            where_clause: where_clause,
            order: order,
            _marker: PhantomData,
        }
    }
}

impl<ST, S, F> SelectStatement<ST, S, F> {
    pub fn simple(select: S, from: F) -> Self {
        SelectStatement::new(select, from, NoWhereClause, NoOrderClause)
    }
}

impl<ST, S, F, W, O> Query for SelectStatement<ST, S, F, W, O> where
    ST: NativeSqlType,
    SelectStatement<ST, S, F, W, O>: QueryFragment
{
    type SqlType = ST;
}

impl<ST, S, F, W, O> QueryFragment for SelectStatement<ST, S, F, W, O> where
    ST: NativeSqlType,
    F: QuerySource,
    S: SelectableExpression<F, ST>,
    W: QueryFragment,
    O: QueryFragment,
{
    fn to_sql<T: QueryBuilder>(&self, out: &mut T) -> BuildQueryResult {
        out.push_sql("SELECT ");
        try!(self.select.to_sql(out));
        out.push_sql(" FROM ");
        try!(self.from.from_clause(out));
        try!(self.where_clause.to_sql(out));
        self.order.to_sql(out)
    }
}
