use std::marker::PhantomData;

use backend::Backend;
use expression::*;
use query_builder::*;
use query_builder::limit_clause::LimitClause;
use query_builder::offset_clause::OffsetClause;
use query_builder::order_clause::OrderClause;
use query_dsl::*;
use query_source::QuerySource;
use result::QueryResult;
use types::{HasSqlType, Bool, BigInt};

#[allow(missing_debug_implementations)]
pub struct BoxedSelectStatement<'a, ST, QS, DB> {
    select: Box<QueryFragment<DB> + 'a>,
    from: QS,
    distinct: Box<QueryFragment<DB> + 'a>,
    where_clause: Option<Box<QueryFragment<DB> + 'a>>,
    order: Box<QueryFragment<DB> + 'a>,
    limit: Box<QueryFragment<DB> + 'a>,
    offset: Box<QueryFragment<DB> + 'a>,
    _marker: PhantomData<ST>,
}

impl<'a, ST, QS, DB> BoxedSelectStatement<'a, ST, QS, DB> {
    pub fn new(
        select: Box<QueryFragment<DB> + 'a>,
        from: QS,
        distinct: Box<QueryFragment<DB> + 'a>,
        where_clause: Option<Box<QueryFragment<DB> + 'a>>,
        order: Box<QueryFragment<DB> + 'a>,
        limit: Box<QueryFragment<DB> + 'a>,
        offset: Box<QueryFragment<DB> + 'a>,
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

impl<'a, ST, QS, DB> Query for BoxedSelectStatement<'a, ST, QS, DB> where
    DB: Backend,
    DB: HasSqlType<ST>,
{
    type SqlType = ST;
}

impl<'a, ST, QS, DB> QueryFragment<DB> for BoxedSelectStatement<'a, ST, QS, DB> where
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

        if let Some(ref where_clause) = self.where_clause {
            out.push_sql(" WHERE ");
            try!(where_clause.to_sql(out));
        }

        try!(self.order.to_sql(out));
        try!(self.limit.to_sql(out));
        try!(self.offset.to_sql(out));
        Ok(())
    }

    fn collect_binds(&self, out: &mut DB::BindCollector) -> QueryResult<()> {
        try!(self.distinct.collect_binds(out));
        try!(self.select.collect_binds(out));
        try!(self.from.from_clause().collect_binds(out));

        if let Some(ref where_clause) = self.where_clause {
            try!(where_clause.collect_binds(out));
        }

        try!(self.order.collect_binds(out));
        try!(self.limit.collect_binds(out));
        try!(self.offset.collect_binds(out));
        Ok(())
    }

    fn is_safe_to_cache_prepared(&self) -> bool {
        self.distinct.is_safe_to_cache_prepared() &&
            self.select.is_safe_to_cache_prepared() &&
            self.from.from_clause().is_safe_to_cache_prepared() &&
            self.where_clause.as_ref().map(|w| w.is_safe_to_cache_prepared()).unwrap_or(true) &&
            self.order.is_safe_to_cache_prepared() &&
            self.limit.is_safe_to_cache_prepared() &&
            self.offset.is_safe_to_cache_prepared()
    }
}

impl<'a, ST, DB> QueryFragment<DB> for BoxedSelectStatement<'a, ST, (), DB> where
    DB: Backend,
{
    fn to_sql(&self, out: &mut DB::QueryBuilder) -> BuildQueryResult {
        out.push_sql("SELECT ");
        try!(self.distinct.to_sql(out));
        try!(self.select.to_sql(out));

        if let Some(ref where_clause) = self.where_clause {
            out.push_sql(" WHERE ");
            try!(where_clause.to_sql(out));
        }

        try!(self.order.to_sql(out));
        try!(self.limit.to_sql(out));
        try!(self.offset.to_sql(out));
        Ok(())
    }

    fn collect_binds(&self, out: &mut DB::BindCollector) -> QueryResult<()> {
        try!(self.distinct.collect_binds(out));
        try!(self.select.collect_binds(out));

        if let Some(ref where_clause) = self.where_clause {
            try!(where_clause.collect_binds(out));
        }

        try!(self.order.collect_binds(out));
        try!(self.limit.collect_binds(out));
        try!(self.offset.collect_binds(out));
        Ok(())
    }

    fn is_safe_to_cache_prepared(&self) -> bool {
        self.distinct.is_safe_to_cache_prepared() &&
            self.select.is_safe_to_cache_prepared() &&
            self.where_clause.as_ref().map(|w| w.is_safe_to_cache_prepared()).unwrap_or(true) &&
            self.order.is_safe_to_cache_prepared() &&
            self.limit.is_safe_to_cache_prepared() &&
            self.offset.is_safe_to_cache_prepared()
    }
}

impl<'a, ST, QS, DB> QueryId for BoxedSelectStatement<'a, ST, QS, DB> {
    type QueryId = ();

    fn has_static_query_id() -> bool {
        false
    }
}

impl<'a, ST, QS, DB, Selection> SelectDsl<Selection, Selection::SqlTypeForSelect>
    for BoxedSelectStatement<'a, ST, QS, DB> where
        DB: Backend + HasSqlType<Selection::SqlTypeForSelect>,
        Selection: SelectableExpression<QS> + QueryFragment<DB> + 'a,
{
    type Output = BoxedSelectStatement<'a, Selection::SqlTypeForSelect, QS, DB>;

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

impl<'a, ST, QS, DB, Predicate> FilterDsl<Predicate>
    for BoxedSelectStatement<'a, ST, QS, DB> where
        DB: Backend + HasSqlType<ST> + 'a,
        Predicate: AppearsOnTable<QS, SqlType=Bool> + NonAggregate,
        Predicate: QueryFragment<DB> + 'a,
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

impl<'a, ST, QS, DB> LimitDsl for BoxedSelectStatement<'a, ST, QS, DB> where
    DB: Backend,
    BoxedSelectStatement<'a, ST, QS, DB>: Query,
{
    type Output = Self;

    fn limit(mut self, limit: i64) -> Self::Output {
        let limit_expression = AsExpression::<BigInt>::as_expression(limit);
        self.limit = Box::new(LimitClause(limit_expression));
        self
    }
}

impl<'a, ST, QS, DB> OffsetDsl for BoxedSelectStatement<'a, ST, QS, DB> where
    DB: Backend,
    BoxedSelectStatement<'a, ST, QS, DB>: Query,
{
    type Output = Self;

    fn offset(mut self, offset: i64) -> Self::Output {
        let offset_expression = AsExpression::<BigInt>::as_expression(offset);
        self.offset = Box::new(OffsetClause(offset_expression));
        self
    }
}

impl<'a, ST, QS, DB, Order> OrderDsl<Order>
    for BoxedSelectStatement<'a, ST, QS, DB> where
        DB: Backend,
        Order: QueryFragment<DB> + AppearsOnTable<QS> + 'a,
        BoxedSelectStatement<'a, ST, QS, DB>: Query,
{
    type Output = Self;

    fn order(mut self, order: Order) -> Self::Output {
        self.order = Box::new(OrderClause(order));
        self
    }
}
