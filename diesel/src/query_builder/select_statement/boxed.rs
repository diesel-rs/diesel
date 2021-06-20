use std::marker::PhantomData;

use crate::backend::Backend;
use crate::dsl::AsExprOf;
use crate::expression::subselect::ValidSubselect;
use crate::expression::*;
use crate::insertable::Insertable;
use crate::query_builder::combination_clause::*;
use crate::query_builder::distinct_clause::DistinctClause;
use crate::query_builder::group_by_clause::ValidGroupByClause;
use crate::query_builder::having_clause::HavingClause;
use crate::query_builder::insert_statement::InsertFromSelect;
use crate::query_builder::limit_clause::LimitClause;
use crate::query_builder::limit_offset_clause::BoxedLimitOffsetClause;
use crate::query_builder::offset_clause::OffsetClause;
use crate::query_builder::order_clause::OrderClause;
use crate::query_builder::where_clause::*;
use crate::query_builder::*;
use crate::query_dsl::methods::*;
use crate::query_dsl::*;
use crate::query_source::joins::*;
use crate::query_source::{QuerySource, Table};
use crate::result::QueryResult;
use crate::sql_types::{BigInt, BoolOrNullableBool, IntoNullable};

#[allow(missing_debug_implementations)]
pub struct BoxedSelectStatement<'a, ST, QS, DB, GB = ()> {
    select: Box<dyn QueryFragment<DB> + Send + 'a>,
    from: QS,
    distinct: Box<dyn QueryFragment<DB> + Send + 'a>,
    where_clause: BoxedWhereClause<'a, DB>,
    order: Option<Box<dyn QueryFragment<DB> + Send + 'a>>,
    limit_offset: BoxedLimitOffsetClause<'a, DB>,
    group_by: Box<dyn QueryFragment<DB> + Send + 'a>,
    having: Box<dyn QueryFragment<DB> + Send + 'a>,
    _marker: PhantomData<(ST, GB)>,
}

impl<'a, ST, QS, DB, GB> BoxedSelectStatement<'a, ST, QS, DB, GB> {
    #[allow(clippy::too_many_arguments)]
    pub fn new<S, G>(
        select: S,
        from: QS,
        distinct: Box<dyn QueryFragment<DB> + Send + 'a>,
        where_clause: BoxedWhereClause<'a, DB>,
        order: Option<Box<dyn QueryFragment<DB> + Send + 'a>>,
        limit_offset: BoxedLimitOffsetClause<'a, DB>,
        group_by: G,
        having: Box<dyn QueryFragment<DB> + Send + 'a>,
    ) -> Self
    where
        DB: Backend,
        G: ValidGroupByClause<Expressions = GB> + QueryFragment<DB> + Send + 'a,
        S: IntoBoxedSelectClause<'a, DB, QS> + SelectClauseExpression<QS>,
        S::Selection: ValidGrouping<GB>,
    {
        BoxedSelectStatement {
            select: select.into_boxed(&from),
            from,
            distinct,
            where_clause,
            order,
            limit_offset,
            group_by: Box::new(group_by),
            having,
            _marker: PhantomData,
        }
    }
}

impl<'a, ST, QS, DB, GB> BoxedSelectStatement<'a, ST, QS, DB, GB> {
    pub(crate) fn build_query(
        &self,
        mut out: AstPass<DB>,
        where_clause_handler: impl Fn(&BoxedWhereClause<'a, DB>, AstPass<DB>) -> QueryResult<()>,
    ) -> QueryResult<()>
    where
        DB: Backend,
        QS: QuerySource,
        QS::FromClause: QueryFragment<DB>,
        BoxedLimitOffsetClause<'a, DB>: QueryFragment<DB>,
    {
        out.push_sql("SELECT ");
        self.distinct.walk_ast(out.reborrow())?;
        self.select.walk_ast(out.reborrow())?;
        out.push_sql(" FROM ");
        self.from.from_clause().walk_ast(out.reborrow())?;
        where_clause_handler(&self.where_clause, out.reborrow())?;
        self.group_by.walk_ast(out.reborrow())?;
        self.having.walk_ast(out.reborrow())?;

        if let Some(ref order) = self.order {
            out.push_sql(" ORDER BY ");
            order.walk_ast(out.reborrow())?;
        }
        self.limit_offset.walk_ast(out.reborrow())?;
        Ok(())
    }
}

impl<'a, ST, QS, DB, GB> Query for BoxedSelectStatement<'a, ST, QS, DB, GB>
where
    DB: Backend,
{
    type SqlType = ST;
}

impl<'a, ST, QS, DB, GB> SelectQuery for BoxedSelectStatement<'a, ST, QS, DB, GB>
where
    DB: Backend,
{
    type SqlType = ST;
}

impl<'a, ST, QS, QS2, DB, GB> ValidSubselect<QS2> for BoxedSelectStatement<'a, ST, QS, DB, GB> where
    Self: Query<SqlType = ST>
{
}

impl<'a, ST, QS, DB, GB> QueryFragment<DB> for BoxedSelectStatement<'a, ST, QS, DB, GB>
where
    DB: Backend,
    QS: QuerySource,
    QS::FromClause: QueryFragment<DB>,
    BoxedLimitOffsetClause<'a, DB>: QueryFragment<DB>,
{
    fn walk_ast(&self, out: AstPass<DB>) -> QueryResult<()> {
        self.build_query(out, |where_clause, out| where_clause.walk_ast(out))
    }
}

impl<'a, ST, DB, GB> QueryFragment<DB> for BoxedSelectStatement<'a, ST, (), DB, GB>
where
    DB: Backend,
    BoxedLimitOffsetClause<'a, DB>: QueryFragment<DB>,
{
    fn walk_ast(&self, mut out: AstPass<DB>) -> QueryResult<()> {
        out.push_sql("SELECT ");
        self.distinct.walk_ast(out.reborrow())?;
        self.select.walk_ast(out.reborrow())?;
        self.where_clause.walk_ast(out.reborrow())?;
        self.group_by.walk_ast(out.reborrow())?;
        self.having.walk_ast(out.reborrow())?;
        self.order.walk_ast(out.reborrow())?;
        self.limit_offset.walk_ast(out.reborrow())?;
        Ok(())
    }
}

impl<'a, ST, QS, DB, GB> QueryId for BoxedSelectStatement<'a, ST, QS, DB, GB> {
    type QueryId = ();

    const HAS_STATIC_QUERY_ID: bool = false;
}

impl<'a, ST, QS, DB, Rhs, Kind, On, GB> InternalJoinDsl<Rhs, Kind, On>
    for BoxedSelectStatement<'a, ST, QS, DB, GB>
where
    BoxedSelectStatement<'a, ST, JoinOn<Join<QS, Rhs, Kind>, On>, DB, GB>: AsQuery,
{
    type Output = BoxedSelectStatement<'a, ST, JoinOn<Join<QS, Rhs, Kind>, On>, DB, GB>;

    fn join(self, rhs: Rhs, kind: Kind, on: On) -> Self::Output {
        BoxedSelectStatement {
            select: self.select,
            from: Join::new(self.from, rhs, kind).on(on),
            distinct: self.distinct,
            where_clause: self.where_clause,
            order: self.order,
            limit_offset: self.limit_offset,
            group_by: self.group_by,
            having: self.having,
            _marker: PhantomData,
        }
    }
}

impl<'a, ST, QS, DB, GB> DistinctDsl for BoxedSelectStatement<'a, ST, QS, DB, GB>
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

impl<'a, ST, QS, DB, Selection, GB> SelectDsl<Selection>
    for BoxedSelectStatement<'a, ST, QS, DB, GB>
where
    DB: Backend,
    Selection: SelectableExpression<QS> + QueryFragment<DB> + ValidGrouping<GB> + Send + 'a,
{
    type Output = BoxedSelectStatement<'a, Selection::SqlType, QS, DB, GB>;

    fn select(self, selection: Selection) -> Self::Output {
        BoxedSelectStatement {
            select: Box::new(selection),
            from: self.from,
            distinct: self.distinct,
            where_clause: self.where_clause,
            order: self.order,
            limit_offset: self.limit_offset,
            group_by: self.group_by,
            having: self.having,
            _marker: PhantomData,
        }
    }
}

impl<'a, ST, QS, DB, Predicate, GB> FilterDsl<Predicate>
    for BoxedSelectStatement<'a, ST, QS, DB, GB>
where
    BoxedWhereClause<'a, DB>: WhereAnd<Predicate, Output = BoxedWhereClause<'a, DB>>,
    Predicate: AppearsOnTable<QS> + NonAggregate,
    Predicate::SqlType: BoolOrNullableBool,
{
    type Output = Self;

    fn filter(mut self, predicate: Predicate) -> Self::Output {
        self.where_clause = self.where_clause.and(predicate);
        self
    }
}

impl<'a, ST, QS, DB, Predicate, GB> OrFilterDsl<Predicate>
    for BoxedSelectStatement<'a, ST, QS, DB, GB>
where
    BoxedWhereClause<'a, DB>: WhereOr<Predicate, Output = BoxedWhereClause<'a, DB>>,
    Predicate: AppearsOnTable<QS> + NonAggregate,
    Predicate::SqlType: BoolOrNullableBool,
{
    type Output = Self;

    fn or_filter(mut self, predicate: Predicate) -> Self::Output {
        self.where_clause = self.where_clause.or(predicate);
        self
    }
}

impl<'a, ST, QS, DB, GB> LimitDsl for BoxedSelectStatement<'a, ST, QS, DB, GB>
where
    DB: Backend,
    LimitClause<AsExprOf<i64, BigInt>>: QueryFragment<DB>,
{
    type Output = Self;

    fn limit(mut self, limit: i64) -> Self::Output {
        self.limit_offset.limit = Some(Box::new(LimitClause(limit.into_sql::<BigInt>())));
        self
    }
}

impl<'a, ST, QS, DB, GB> OffsetDsl for BoxedSelectStatement<'a, ST, QS, DB, GB>
where
    DB: Backend,
    OffsetClause<AsExprOf<i64, BigInt>>: QueryFragment<DB>,
{
    type Output = Self;

    fn offset(mut self, offset: i64) -> Self::Output {
        self.limit_offset.offset = Some(Box::new(OffsetClause(offset.into_sql::<BigInt>())));
        self
    }
}

impl<'a, ST, QS, DB, Order, GB> OrderDsl<Order> for BoxedSelectStatement<'a, ST, QS, DB, GB>
where
    DB: Backend,
    Order: QueryFragment<DB> + AppearsOnTable<QS> + Send + 'a,
{
    type Output = Self;

    fn order(mut self, order: Order) -> Self::Output {
        self.order = OrderClause(order).into();
        self
    }
}

impl<'a, ST, QS, DB, Order, GB> ThenOrderDsl<Order> for BoxedSelectStatement<'a, ST, QS, DB, GB>
where
    DB: Backend + 'a,
    Order: QueryFragment<DB> + AppearsOnTable<QS> + Send + 'a,
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

impl<'a, ST, QS, DB, Rhs> JoinTo<Rhs> for BoxedSelectStatement<'a, ST, QS, DB, ()>
where
    QS: JoinTo<Rhs>,
{
    type FromClause = QS::FromClause;
    type OnClause = QS::OnClause;

    fn join_target(rhs: Rhs) -> (Self::FromClause, Self::OnClause) {
        QS::join_target(rhs)
    }
}

impl<'a, ST, QS, DB, GB> QueryDsl for BoxedSelectStatement<'a, ST, QS, DB, GB> {}

impl<'a, ST, QS, DB, Conn, GB> RunQueryDsl<Conn> for BoxedSelectStatement<'a, ST, QS, DB, GB> {}

impl<'a, ST, QS, DB, T, GB> Insertable<T> for BoxedSelectStatement<'a, ST, QS, DB, GB>
where
    T: Table,
    Self: Query,
    <T::AllColumns as ValidGrouping<()>>::IsAggregate:
        MixedAggregates<is_aggregate::No, Output = is_aggregate::No>,
{
    type Values = InsertFromSelect<Self, T::AllColumns>;

    fn values(self) -> Self::Values {
        InsertFromSelect::new(self)
    }
}

impl<'a, 'b, ST, QS, DB, T, GB> Insertable<T> for &'b BoxedSelectStatement<'a, ST, QS, DB, GB>
where
    T: Table,
    Self: Query,
    <T::AllColumns as ValidGrouping<()>>::IsAggregate:
        MixedAggregates<is_aggregate::No, Output = is_aggregate::No>,
{
    type Values = InsertFromSelect<Self, T::AllColumns>;

    fn values(self) -> Self::Values {
        InsertFromSelect::new(self)
    }
}

impl<'a, ST, QS, DB, GB> SelectNullableDsl for BoxedSelectStatement<'a, ST, QS, DB, GB>
where
    ST: IntoNullable,
{
    type Output = BoxedSelectStatement<'a, ST::Nullable, QS, DB>;

    fn nullable(self) -> Self::Output {
        BoxedSelectStatement {
            select: self.select,
            from: self.from,
            distinct: self.distinct,
            where_clause: self.where_clause,
            order: self.order,
            limit_offset: self.limit_offset,
            group_by: self.group_by,
            having: self.having,
            _marker: PhantomData,
        }
    }
}

impl<'a, ST, QS, DB, GB, Predicate> HavingDsl<Predicate>
    for BoxedSelectStatement<'a, ST, QS, DB, GB>
where
    DB: Backend,
    GB: Expression,
    HavingClause<Predicate>: QueryFragment<DB> + Send + 'a,
    Predicate: AppearsOnTable<QS>,
    Predicate::SqlType: BoolOrNullableBool,
{
    type Output = Self;

    fn having(mut self, predicate: Predicate) -> Self::Output {
        self.having = Box::new(HavingClause(predicate));
        self
    }
}

impl<'a, ST, QS, DB, GB> CombineDsl for BoxedSelectStatement<'a, ST, QS, DB, GB>
where
    Self: Query,
{
    type Query = Self;

    fn union<Rhs>(self, rhs: Rhs) -> crate::dsl::Union<Self, Rhs>
    where
        Rhs: AsQuery<SqlType = <Self::Query as Query>::SqlType>,
    {
        CombinationClause::new(Union, Distinct, self, rhs.as_query())
    }

    fn union_all<Rhs>(self, rhs: Rhs) -> crate::dsl::UnionAll<Self, Rhs>
    where
        Rhs: AsQuery<SqlType = <Self::Query as Query>::SqlType>,
    {
        CombinationClause::new(Union, All, self, rhs.as_query())
    }

    fn intersect<Rhs>(self, rhs: Rhs) -> crate::dsl::Intersect<Self, Rhs>
    where
        Rhs: AsQuery<SqlType = <Self::Query as Query>::SqlType>,
    {
        CombinationClause::new(Intersect, Distinct, self, rhs.as_query())
    }

    fn intersect_all<Rhs>(self, rhs: Rhs) -> crate::dsl::IntersectAll<Self, Rhs>
    where
        Rhs: AsQuery<SqlType = <Self::Query as Query>::SqlType>,
    {
        CombinationClause::new(Intersect, All, self, rhs.as_query())
    }

    fn except<Rhs>(self, rhs: Rhs) -> crate::dsl::Except<Self, Rhs>
    where
        Rhs: AsQuery<SqlType = <Self::Query as Query>::SqlType>,
    {
        CombinationClause::new(Except, Distinct, self, rhs.as_query())
    }

    fn except_all<Rhs>(self, rhs: Rhs) -> crate::dsl::ExceptAll<Self, Rhs>
    where
        Rhs: AsQuery<SqlType = <Self::Query as Query>::SqlType>,
    {
        CombinationClause::new(Except, All, self, rhs.as_query())
    }
}

#[cfg(test)]
mod tests {
    use crate::prelude::*;

    table! {
        users {
            id -> Integer,
        }
    }

    fn assert_send<T>(_: T)
    where
        T: Send,
    {
    }

    macro_rules! assert_boxed_query_send {
        ($backend:ty) => {{
            assert_send(users::table.into_boxed::<$backend>());
            assert_send(
                users::table
                    .filter(users::id.eq(10))
                    .into_boxed::<$backend>(),
            );
        };};
    }

    #[test]
    fn boxed_is_send() {
        #[cfg(feature = "postgres")]
        assert_boxed_query_send!(crate::pg::Pg);

        #[cfg(feature = "sqlite")]
        assert_boxed_query_send!(crate::sqlite::Sqlite);

        #[cfg(feature = "mysql")]
        assert_boxed_query_send!(crate::mysql::Mysql);
    }
}
