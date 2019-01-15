use std::marker::PhantomData;

use backend::Backend;
use dsl::AsExprOf;
use expression::subselect::ValidSubselect;
use expression::*;
use insertable::Insertable;
use query_builder::distinct_clause::DistinctClause;
use query_builder::group_by_clause::GroupByClause;
use query_builder::insert_statement::InsertFromSelect;
use query_builder::limit_clause::LimitClause;
use query_builder::offset_clause::OffsetClause;
use query_builder::order_clause::OrderClause;
use query_builder::where_clause::*;
use query_builder::*;
use query_dsl::methods::*;
use query_dsl::*;
use query_source::joins::*;
use query_source::{QuerySource, Table};
use result::QueryResult;
use sql_types::{BigInt, Bool, NotNull, Nullable};

#[allow(missing_debug_implementations)]
pub struct BoxedSelectStatement<'a, ST, QS, DB> {
    select: Box<QueryFragment<DB> + 'a>,
    from: QS,
    distinct: Box<QueryFragment<DB> + 'a>,
    where_clause: BoxedWhereClause<'a, DB>,
    order: Option<Box<QueryFragment<DB> + 'a>>,
    limit: Box<QueryFragment<DB> + 'a>,
    offset: Box<QueryFragment<DB> + 'a>,
    group_by: Box<QueryFragment<DB> + 'a>,
    _marker: PhantomData<ST>,
}

impl<'a, ST, QS, DB> BoxedSelectStatement<'a, ST, QS, DB> {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        select: Box<QueryFragment<DB> + 'a>,
        from: QS,
        distinct: Box<QueryFragment<DB> + 'a>,
        where_clause: BoxedWhereClause<'a, DB>,
        order: Option<Box<QueryFragment<DB> + 'a>>,
        limit: Box<QueryFragment<DB> + 'a>,
        offset: Box<QueryFragment<DB> + 'a>,
        group_by: Box<QueryFragment<DB> + 'a>,
    ) -> Self {
        BoxedSelectStatement {
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
}

impl<'a, ST, QS, DB> Query for BoxedSelectStatement<'a, ST, QS, DB>
where
    DB: Backend,
{
    type SqlType = ST;
}

impl<'a, ST, QS, DB> SelectQuery for BoxedSelectStatement<'a, ST, QS, DB>
where
    DB: Backend,
{
    type SqlType = ST;
}

impl<'a, ST, QS, QS2, DB> ValidSubselect<QS2> for BoxedSelectStatement<'a, ST, QS, DB> where
    Self: Query<SqlType = ST>
{
}

impl<'a, ST, QS, DB> QueryFragment<DB> for BoxedSelectStatement<'a, ST, QS, DB>
where
    DB: Backend,
    QS: QuerySource,
    QS::FromClause: QueryFragment<DB>,
{
    fn walk_ast(&self, mut out: AstPass<DB>) -> QueryResult<()> {
        out.push_sql("SELECT ");
        self.distinct.walk_ast(out.reborrow())?;
        self.select.walk_ast(out.reborrow())?;
        out.push_sql(" FROM ");
        self.from.from_clause().walk_ast(out.reborrow())?;
        self.where_clause.walk_ast(out.reborrow())?;
        self.group_by.walk_ast(out.reborrow())?;

        if let Some(ref order) = self.order {
            out.push_sql(" ORDER BY ");
            order.walk_ast(out.reborrow())?;
        }

        self.limit.walk_ast(out.reborrow())?;
        self.offset.walk_ast(out.reborrow())?;
        Ok(())
    }
}

impl<'a, ST, DB> QueryFragment<DB> for BoxedSelectStatement<'a, ST, (), DB>
where
    DB: Backend,
{
    fn walk_ast(&self, mut out: AstPass<DB>) -> QueryResult<()> {
        out.push_sql("SELECT ");
        self.distinct.walk_ast(out.reborrow())?;
        self.select.walk_ast(out.reborrow())?;
        self.where_clause.walk_ast(out.reborrow())?;
        self.group_by.walk_ast(out.reborrow())?;
        self.order.walk_ast(out.reborrow())?;
        self.limit.walk_ast(out.reborrow())?;
        self.offset.walk_ast(out.reborrow())?;
        Ok(())
    }
}

impl<'a, ST, QS, DB> QueryId for BoxedSelectStatement<'a, ST, QS, DB> {
    type QueryId = ();

    const HAS_STATIC_QUERY_ID: bool = false;
}

impl<'a, ST, QS, DB, Rhs, Kind, On> InternalJoinDsl<Rhs, Kind, On>
    for BoxedSelectStatement<'a, ST, QS, DB>
where
    BoxedSelectStatement<'a, ST, JoinOn<Join<QS, Rhs, Kind>, On>, DB>: AsQuery,
{
    type Output = BoxedSelectStatement<'a, ST, JoinOn<Join<QS, Rhs, Kind>, On>, DB>;

    fn join(self, rhs: Rhs, kind: Kind, on: On) -> Self::Output {
        BoxedSelectStatement::new(
            self.select,
            Join::new(self.from, rhs, kind).on(on),
            self.distinct,
            self.where_clause,
            self.order,
            self.limit,
            self.offset,
            self.group_by,
        )
    }
}

impl<'a, ST, QS, DB> DistinctDsl for BoxedSelectStatement<'a, ST, QS, DB>
where
    DB: Backend,
    DistinctClause: QueryFragment<DB>,
{
    type Output = Self;

    fn distinct(mut self) -> Self::Output {
        self.distinct = Box::new(DistinctClause);
        self
    }
}

impl<'a, ST, QS, DB, Selection> SelectDsl<Selection> for BoxedSelectStatement<'a, ST, QS, DB>
where
    DB: Backend,
    Selection: SelectableExpression<QS> + QueryFragment<DB> + 'a,
{
    type Output = BoxedSelectStatement<'a, Selection::SqlType, QS, DB>;

    fn select(self, selection: Selection) -> Self::Output {
        BoxedSelectStatement::new(
            Box::new(selection),
            self.from,
            self.distinct,
            self.where_clause,
            self.order,
            self.limit,
            self.offset,
            self.group_by,
        )
    }
}

impl<'a, ST, QS, DB, Predicate> FilterDsl<Predicate> for BoxedSelectStatement<'a, ST, QS, DB>
where
    BoxedWhereClause<'a, DB>: WhereAnd<Predicate, Output = BoxedWhereClause<'a, DB>>,
    Predicate: AppearsOnTable<QS, SqlType = Bool> + NonAggregate,
{
    type Output = Self;

    fn filter(mut self, predicate: Predicate) -> Self::Output {
        self.where_clause = self.where_clause.and(predicate);
        self
    }
}

impl<'a, ST, QS, DB, Predicate> OrFilterDsl<Predicate> for BoxedSelectStatement<'a, ST, QS, DB>
where
    BoxedWhereClause<'a, DB>: WhereOr<Predicate, Output = BoxedWhereClause<'a, DB>>,
    Predicate: AppearsOnTable<QS, SqlType = Bool> + NonAggregate,
{
    type Output = Self;

    fn or_filter(mut self, predicate: Predicate) -> Self::Output {
        self.where_clause = self.where_clause.or(predicate);
        self
    }
}

impl<'a, ST, QS, DB> LimitDsl for BoxedSelectStatement<'a, ST, QS, DB>
where
    DB: Backend,
    LimitClause<AsExprOf<i64, BigInt>>: QueryFragment<DB>,
{
    type Output = Self;

    fn limit(mut self, limit: i64) -> Self::Output {
        self.limit = Box::new(LimitClause(limit.into_sql::<BigInt>()));
        self
    }
}

impl<'a, ST, QS, DB> OffsetDsl for BoxedSelectStatement<'a, ST, QS, DB>
where
    DB: Backend,
    OffsetClause<AsExprOf<i64, BigInt>>: QueryFragment<DB>,
{
    type Output = Self;

    fn offset(mut self, offset: i64) -> Self::Output {
        self.offset = Box::new(OffsetClause(offset.into_sql::<BigInt>()));
        self
    }
}

impl<'a, ST, QS, DB, Order> OrderDsl<Order> for BoxedSelectStatement<'a, ST, QS, DB>
where
    DB: Backend,
    Order: QueryFragment<DB> + AppearsOnTable<QS> + 'a,
{
    type Output = Self;

    fn order(mut self, order: Order) -> Self::Output {
        self.order = OrderClause(order).into();
        self
    }
}

impl<'a, ST, QS, DB, Order> ThenOrderDsl<Order> for BoxedSelectStatement<'a, ST, QS, DB>
where
    DB: Backend + 'a,
    Order: QueryFragment<DB> + AppearsOnTable<QS> + 'a,
{
    type Output = Self;

    fn then_order_by(mut self, order: Order) -> Self::Output {
        self.order = match self.order {
            Some(old) => Some(Box::new((old, order))),
            None => Some(Box::new(order)),
        };
        self
    }
}

impl<'a, ST, QS, DB, Expr> GroupByDsl<Expr> for BoxedSelectStatement<'a, ST, QS, DB>
where
    DB: Backend,
    Expr: QueryFragment<DB> + AppearsOnTable<QS> + 'a,
    Self: Query,
{
    type Output = Self;

    fn group_by(mut self, group_by: Expr) -> Self::Output {
        self.group_by = Box::new(GroupByClause(group_by));
        self
    }
}

// FIXME: Should we disable joining when `.group_by` has been called? Are there
// any other query methods where a join no longer has the same semantics as
// joining on just the table?
impl<'a, ST, QS, DB, Rhs> JoinTo<Rhs> for BoxedSelectStatement<'a, ST, QS, DB>
where
    QS: JoinTo<Rhs>,
{
    type FromClause = QS::FromClause;
    type OnClause = QS::OnClause;

    fn join_target(rhs: Rhs) -> (Self::FromClause, Self::OnClause) {
        QS::join_target(rhs)
    }
}

impl<'a, ST, QS, DB> QueryDsl for BoxedSelectStatement<'a, ST, QS, DB> {}

impl<'a, ST, QS, DB, Conn> RunQueryDsl<Conn> for BoxedSelectStatement<'a, ST, QS, DB> {}

impl<'a, ST, QS, DB, T> Insertable<T> for BoxedSelectStatement<'a, ST, QS, DB>
where
    T: Table,
    Self: Query,
{
    type Values = InsertFromSelect<Self, T::AllColumns>;

    fn values(self) -> Self::Values {
        InsertFromSelect::new(self)
    }
}

impl<'a, 'b, ST, QS, DB, T> Insertable<T> for &'b BoxedSelectStatement<'a, ST, QS, DB>
where
    T: Table,
    Self: Query,
{
    type Values = InsertFromSelect<Self, T::AllColumns>;

    fn values(self) -> Self::Values {
        InsertFromSelect::new(self)
    }
}

impl<'a, ST, QS, DB> SelectNullableDsl for BoxedSelectStatement<'a, ST, QS, DB>
where
    ST: NotNull,
{
    type Output = BoxedSelectStatement<'a, Nullable<ST>, QS, DB>;

    fn nullable(self) -> Self::Output {
        BoxedSelectStatement {
            select: self.select,
            from: self.from,
            distinct: self.distinct,
            where_clause: self.where_clause,
            order: self.order,
            limit: self.limit,
            offset: self.offset,
            group_by: self.group_by,
            _marker: PhantomData,
        }
    }
}
