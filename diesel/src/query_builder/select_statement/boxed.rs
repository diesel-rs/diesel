use std::marker::PhantomData;

use backend::Backend;
use query_builder::*;
use query_source::QuerySource;
use types::HasSqlType;

pub struct BoxedSelectStatement<ST, QS, DB> {
    select: Box<QueryFragment<DB>>,
    from: QS,
    where_clause: Box<QueryFragment<DB>>,
    order: Box<QueryFragment<DB>>,
    limit: Box<QueryFragment<DB>>,
    offset: Box<QueryFragment<DB>>,
    _marker: PhantomData<(ST, DB)>,
}

impl<ST, QS, DB> BoxedSelectStatement<ST, QS, DB> {
    pub fn new(
        select: Box<QueryFragment<DB>>,
        from: QS,
        where_clause: Box<QueryFragment<DB>>,
        order: Box<QueryFragment<DB>>,
        limit: Box<QueryFragment<DB>>,
        offset: Box<QueryFragment<DB>>,
    ) -> Self {
        BoxedSelectStatement {
            select: select,
            from: from,
            where_clause: where_clause,
            order: order,
            limit: limit,
            offset: offset,
            _marker: PhantomData,
        }
    }
}

impl<ST, QS, DB> Query for BoxedSelectStatement<ST, QS, DB> where
    DB: Backend,
    DB: HasSqlType<ST>,
{
    type SqlType = ST;
}

impl<ST, QS, DB> QueryFragment<DB> for BoxedSelectStatement<ST, QS, DB> where
    DB: Backend,
    QS: QuerySource,
    QS::FromClause: QueryFragment<DB>,
{
    fn to_sql(&self, out: &mut DB::QueryBuilder) -> BuildQueryResult {
        out.push_sql("SELECT ");
        try!(self.select.to_sql(out));
        out.push_sql(" FROM ");
        try!(self.from.from_clause().to_sql(out));
        try!(self.where_clause.to_sql(out));
        try!(self.order.to_sql(out));
        try!(self.limit.to_sql(out));
        try!(self.offset.to_sql(out));
        Ok(())
    }
}
