use std::marker::PhantomData;

use backend::Backend;
use expression::{SelectableExpression, NonAggregate, AsExpression};
use query_builder::*;
use query_builder::limit_clause::LimitClause;
use query_builder::offset_clause::OffsetClause;
use query_dsl::*;
use query_source::QuerySource;
use types::{HasSqlType, Bool, BigInt};

pub struct BoxedSelectStatement<ST, QS, DB> {
    select: Box<QueryFragment<DB>>,
    from: QS,
    distinct: Box<QueryFragment<DB>>,
    where_clause: Option<Box<QueryFragment<DB>>>,
    order: Box<QueryFragment<DB>>,
    limit: Box<QueryFragment<DB>>,
    offset: Box<QueryFragment<DB>>,
    _marker: PhantomData<(ST, DB)>,
}

impl<ST, QS, DB> BoxedSelectStatement<ST, QS, DB> {
    pub fn new(
        select: Box<QueryFragment<DB>>,
        from: QS,
        distinct: Box<QueryFragment<DB>>,
        where_clause: Option<Box<QueryFragment<DB>>>,
        order: Box<QueryFragment<DB>>,
        limit: Box<QueryFragment<DB>>,
        offset: Box<QueryFragment<DB>>,
    ) -> Self {
        BoxedSelectStatement {
            select: select,
            from: from,
            distinct: distinct,
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
        try!(self.distinct.to_sql(out));
        try!(self.select.to_sql(out));
        out.push_sql(" FROM ");
        try!(self.from.from_clause().to_sql(out));

        match self.where_clause {
            Some(ref where_clause) => {
                out.push_sql(" WHERE ");
                try!(where_clause.to_sql(out));
            }
            None => {}
        }

        try!(self.order.to_sql(out));
        try!(self.limit.to_sql(out));
        try!(self.offset.to_sql(out));
        Ok(())
    }
}

impl<ST, QS, DB, Type, Selection> SelectDsl<Selection, Type>
    for BoxedSelectStatement<ST, QS, DB> where
        DB: Backend + HasSqlType<Type>,
        Selection: SelectableExpression<QS, Type> + QueryFragment<DB> + 'static,
{
    type Output = BoxedSelectStatement<Type, QS, DB>;

    fn select(self, selection: Selection) -> Self::Output {
        BoxedSelectStatement::new(
            Box::new(selection),
            self.from,
            self.distinct,
            self.where_clause,
            self.order,
            self.limit,
            self.offset,
        )
    }
}

impl<ST, QS, DB, Predicate> FilterDsl<Predicate>
    for BoxedSelectStatement<ST, QS, DB> where
        DB: Backend + HasSqlType<ST> + 'static,
        Predicate: SelectableExpression<QS, SqlType=Bool> + NonAggregate,
        Predicate: QueryFragment<DB> + 'static,
{
    type Output = Self;

    fn filter(mut self, predicate: Predicate) -> Self::Output {
        use expression::predicates::And;
        self.where_clause = Some(match self.where_clause {
            Some(where_clause) => Box::new(And::new(where_clause, predicate)),
            None => Box::new(predicate),
        });
        self
    }
}

impl<ST, QS, DB> LimitDsl for BoxedSelectStatement<ST, QS, DB> where
    DB: Backend,
    BoxedSelectStatement<ST, QS, DB>: Query,
{
    type Output = Self;

    fn limit(mut self, limit: i64) -> Self::Output {
        let limit_expression = AsExpression::<BigInt>::as_expression(limit);
        self.limit = Box::new(LimitClause(limit_expression));
        self
    }
}

impl<ST, QS, DB> OffsetDsl for BoxedSelectStatement<ST, QS, DB> where
    DB: Backend,
    BoxedSelectStatement<ST, QS, DB>: Query,
{
    type Output = Self;

    fn offset(mut self, offset: i64) -> Self::Output {
        let offset_expression = AsExpression::<BigInt>::as_expression(offset);
        self.offset = Box::new(OffsetClause(offset_expression));
        self
    }
}
