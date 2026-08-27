use core::marker::PhantomData;

use crate::backend::{DieselReserveSpecialization, sql_dialect};
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
use crate::query_builder::limit_offset_clause::BoxedCloneLimitOffsetClause;
use crate::query_builder::offset_clause::OffsetClause;
use crate::query_builder::order_clause::OrderClause;
use crate::query_builder::where_clause::{BoxedCloneWhereClause, WhereAnd, WhereOr};
use crate::query_builder::*;
use crate::query_dsl::methods::*;
use crate::query_dsl::*;
use crate::query_source::joins::*;
use crate::query_source::{QuerySource, Table};
use crate::sql_types::{BigInt, BoolOrNullableBool, IntoNullable};
use alloc::sync::Arc;
use alloc::vec::Vec;

// This is used by the table macro internally
/// This type represents a boxed select query
///
/// Using this type directly is only meaningful for custom backends
/// that need to provide a custom [`QueryFragment`] implementation
#[allow(missing_debug_implementations)]
#[diesel_derives::__diesel_public_if(
    feature = "i-implement-a-third-party-backend-and-opt-into-breaking-changes",
    public_fields(
        select,
        from,
        distinct,
        where_clause,
        order,
        limit_offset,
        group_by,
        having
    )
)]
pub struct BoxedCloneSelectStatement<'a, ST, QS, DB, GB = ()> {
    /// The select clause of the query
    select: Arc<dyn QueryFragment<DB> + Send + Sync + 'a>,
    /// The from clause of the query
    from: QS,
    /// The distinct clause of the query
    distinct: Arc<dyn QueryFragment<DB> + Send + Sync + 'a>,
    /// The where clause of the query
    where_clause: BoxedCloneWhereClause<'a, DB>,
    /// The order clause of the query
    order: Option<Arc<dyn QueryFragment<DB> + Send + Sync + 'a>>,
    /// The combined limit/offset clause of the query
    limit_offset: BoxedCloneLimitOffsetClause<'a, DB>,
    /// The group by clause of the query
    group_by: Arc<dyn QueryFragment<DB> + Send + Sync + 'a>,
    /// The having clause of the query
    having: Arc<dyn QueryFragment<DB> + Send + Sync + 'a>,
    _marker: PhantomData<(ST, GB)>,
}

impl<ST, QS: Clone, DB, GB> Clone for BoxedCloneSelectStatement<'_, ST, QS, DB, GB> {
    fn clone(&self) -> Self {
        Self {
            select: Arc::clone(&self.select),
            from: self.from.clone(),
            distinct: Arc::clone(&self.distinct),
            where_clause: self.where_clause.clone(),
            order: self.order.as_ref().map(Arc::clone),
            limit_offset: self.limit_offset.clone(),
            group_by: Arc::clone(&self.group_by),
            having: Arc::clone(&self.having),
            _marker: PhantomData,
        }
    }
}

impl<'a, ST, QS: QuerySource, DB, GB> BoxedCloneSelectStatement<'a, ST, FromClause<QS>, DB, GB> {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new<S, G>(
        select: S,
        from: FromClause<QS>,
        distinct: Arc<dyn QueryFragment<DB> + Send + Sync + 'a>,
        where_clause: BoxedCloneWhereClause<'a, DB>,
        order: Option<Arc<dyn QueryFragment<DB> + Send + Sync + 'a>>,
        limit_offset: BoxedCloneLimitOffsetClause<'a, DB>,
        group_by: G,
        having: Arc<dyn QueryFragment<DB> + Send + Sync + 'a>,
    ) -> Self
    where
        DB: Backend,
        G: ValidGroupByClause<Expressions = GB> + QueryFragment<DB> + Send + Sync + 'a,
        S: SelectClauseExpression<FromClause<QS>, SelectClauseSqlType = ST>
            + QueryFragment<DB>
            + Send
            + Sync
            + 'a,
        S::Selection: ValidGrouping<GB>,
    {
        BoxedCloneSelectStatement {
            select: Arc::new(select),
            from,
            distinct,
            where_clause,
            order,
            limit_offset,
            group_by: Arc::new(group_by),
            having,
            _marker: PhantomData,
        }
    }
}

impl<'a, ST, DB, GB> BoxedCloneSelectStatement<'a, ST, NoFromClause, DB, GB> {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new_no_from_clause<S, G>(
        select: S,
        from: NoFromClause,
        distinct: Arc<dyn QueryFragment<DB> + Send + Sync + 'a>,
        where_clause: BoxedCloneWhereClause<'a, DB>,
        order: Option<Arc<dyn QueryFragment<DB> + Send + Sync + 'a>>,
        limit_offset: BoxedCloneLimitOffsetClause<'a, DB>,
        group_by: G,
        having: Arc<dyn QueryFragment<DB> + Send + Sync + 'a>,
    ) -> Self
    where
        DB: Backend,
        G: ValidGroupByClause<Expressions = GB> + QueryFragment<DB> + Send + Sync + 'a,
        S: SelectClauseExpression<NoFromClause, SelectClauseSqlType = ST>
            + QueryFragment<DB>
            + Send
            + Sync
            + 'a,
        S::Selection: ValidGrouping<GB>,
    {
        BoxedCloneSelectStatement {
            select: Arc::new(select),
            from,
            distinct,
            where_clause,
            order,
            limit_offset,
            group_by: Arc::new(group_by),
            having,
            _marker: PhantomData,
        }
    }
}

// that's a trait to control who can access these methods
#[doc(hidden)] // exported via internal::derives::multiconnection
pub trait BoxedCloneQueryHelper<'a, QS, DB> {
    fn build_query<'b, 'c>(
        &'b self,
        out: AstPass<'_, 'c, DB>,
        where_clause_handler: impl Fn(
            &'b BoxedCloneWhereClause<'a, DB>,
            AstPass<'_, 'c, DB>,
        ) -> QueryResult<()>,
    ) -> QueryResult<()>
    where
        DB: Backend,
        QS: QueryFragment<DB>,
        BoxedCloneLimitOffsetClause<'a, DB>: QueryFragment<DB>,
        'b: 'c;
}

impl<'a, ST, QS, DB, GB> BoxedCloneQueryHelper<'a, QS, DB>
    for BoxedCloneSelectStatement<'a, ST, QS, DB, GB>
{
    fn build_query<'b, 'c>(
        &'b self,
        mut out: AstPass<'_, 'c, DB>,
        where_clause_handler: impl Fn(
            &'b BoxedCloneWhereClause<'a, DB>,
            AstPass<'_, 'c, DB>,
        ) -> QueryResult<()>,
    ) -> QueryResult<()>
    where
        DB: Backend,
        QS: QueryFragment<DB>,
        BoxedCloneLimitOffsetClause<'a, DB>: QueryFragment<DB>,
        'b: 'c,
    {
        out.push_sql("SELECT ");
        self.distinct.walk_ast(out.reborrow())?;
        self.select.walk_ast(out.reborrow())?;
        self.from.walk_ast(out.reborrow())?;
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

impl<ST, QS, DB, GB> Query for BoxedCloneSelectStatement<'_, ST, QS, DB, GB>
where
    DB: Backend,
{
    type SqlType = ST;
}

impl<ST, QS, DB, GB> SelectQuery for BoxedCloneSelectStatement<'_, ST, QS, DB, GB>
where
    DB: Backend,
{
    type SqlType = ST;
}

impl<ST, QS, QS2, DB, GB> ValidSubselect<QS2> for BoxedCloneSelectStatement<'_, ST, QS, DB, GB> where
    Self: Query<SqlType = ST>
{
}

impl<ST, QS, DB, GB> QueryFragment<DB> for BoxedCloneSelectStatement<'_, ST, QS, DB, GB>
where
    DB: Backend,
    Self: QueryFragment<DB, DB::SelectStatementSyntax>,
{
    fn walk_ast<'b>(&'b self, pass: AstPass<'_, 'b, DB>) -> QueryResult<()> {
        <Self as QueryFragment<DB, DB::SelectStatementSyntax>>::walk_ast(self, pass)
    }

    fn collect_output_metadata<'b>(
        &'b self,
        out: &mut Vec<OutputFieldMetadata<'b>>,
    ) -> QueryResult<()> {
        self.select.collect_output_metadata(out)
    }
}

impl<'a, ST, QS, DB, GB>
    QueryFragment<DB, sql_dialect::select_statement_syntax::AnsiSqlSelectStatement>
    for BoxedCloneSelectStatement<'a, ST, QS, DB, GB>
where
    DB: Backend<
            SelectStatementSyntax = sql_dialect::select_statement_syntax::AnsiSqlSelectStatement,
        > + DieselReserveSpecialization,
    QS: QueryFragment<DB>,
    BoxedCloneLimitOffsetClause<'a, DB>: QueryFragment<DB>,
{
    fn walk_ast<'b>(&'b self, out: AstPass<'_, 'b, DB>) -> QueryResult<()> {
        self.build_query(out, |where_clause, out| where_clause.walk_ast(out))
    }
}

impl<ST, QS, DB, GB> QueryId for BoxedCloneSelectStatement<'_, ST, QS, DB, GB> {
    type QueryId = ();

    const HAS_STATIC_QUERY_ID: bool = false;
}

impl<'a, ST, QS, DB, Rhs, Kind, On, GB> InternalJoinDsl<Rhs, Kind, On>
    for BoxedCloneSelectStatement<'a, ST, FromClause<QS>, DB, GB>
where
    QS: QuerySource,
    Rhs: QuerySource,
    JoinOn<Join<QS, Rhs, Kind>, On>: QuerySource,
    BoxedCloneSelectStatement<'a, ST, FromClause<JoinOn<Join<QS, Rhs, Kind>, On>>, DB, GB>: AsQuery,
{
    type Output =
        BoxedCloneSelectStatement<'a, ST, FromClause<JoinOn<Join<QS, Rhs, Kind>, On>>, DB, GB>;

    fn join(self, rhs: Rhs, kind: Kind, on: On) -> Self::Output {
        BoxedCloneSelectStatement {
            select: self.select,
            from: FromClause::new(Join::new(self.from.source, rhs, kind).on(on)),
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

impl<ST, QS, DB, GB> DistinctDsl for BoxedCloneSelectStatement<'_, ST, QS, DB, GB>
where
    DB: Backend,
    DistinctClause: QueryFragment<DB>,
{
    type Output = Self;

    fn distinct(mut self) -> Self::Output {
        self.distinct = Arc::new(DistinctClause);
        self
    }
}

impl<'a, ST, QS, DB, Selection, GB> SelectDsl<Selection>
    for BoxedCloneSelectStatement<'a, ST, FromClause<QS>, DB, GB>
where
    DB: Backend,
    QS: QuerySource,
    Selection: SelectableExpression<QS> + QueryFragment<DB> + ValidGrouping<GB> + Send + Sync + 'a,
{
    type Output = BoxedCloneSelectStatement<'a, Selection::SqlType, FromClause<QS>, DB, GB>;

    fn select(self, selection: Selection) -> Self::Output {
        BoxedCloneSelectStatement {
            select: Arc::new(selection),
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

impl<'a, ST, DB, Selection, GB> SelectDsl<Selection>
    for BoxedCloneSelectStatement<'a, ST, NoFromClause, DB, GB>
where
    DB: Backend,
    Selection: SelectableExpression<NoFromClause>
        + QueryFragment<DB>
        + ValidGrouping<GB>
        + Send
        + Sync
        + 'a,
{
    type Output = BoxedCloneSelectStatement<'a, Selection::SqlType, NoFromClause, DB, GB>;

    fn select(self, selection: Selection) -> Self::Output {
        BoxedCloneSelectStatement {
            select: Arc::new(selection),
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
    for BoxedCloneSelectStatement<'a, ST, FromClause<QS>, DB, GB>
where
    QS: QuerySource,
    BoxedCloneWhereClause<'a, DB>: WhereAnd<Predicate, Output = BoxedCloneWhereClause<'a, DB>>,
    Predicate: AppearsOnTable<QS> + NonAggregate,
    Predicate::SqlType: BoolOrNullableBool,
{
    type Output = Self;

    fn filter(mut self, predicate: Predicate) -> Self::Output {
        self.where_clause = self.where_clause.and(predicate);
        self
    }
}

impl<'a, ST, DB, Predicate, GB> FilterDsl<Predicate>
    for BoxedCloneSelectStatement<'a, ST, NoFromClause, DB, GB>
where
    BoxedCloneWhereClause<'a, DB>: WhereAnd<Predicate, Output = BoxedCloneWhereClause<'a, DB>>,
    Predicate: AppearsOnTable<NoFromClause> + NonAggregate,
    Predicate::SqlType: BoolOrNullableBool,
{
    type Output = Self;

    fn filter(mut self, predicate: Predicate) -> Self::Output {
        self.where_clause = self.where_clause.and(predicate);
        self
    }
}

impl<'a, ST, QS, DB, Predicate, GB> OrFilterDsl<Predicate>
    for BoxedCloneSelectStatement<'a, ST, FromClause<QS>, DB, GB>
where
    QS: QuerySource,
    BoxedCloneWhereClause<'a, DB>: WhereOr<Predicate, Output = BoxedCloneWhereClause<'a, DB>>,
    Predicate: AppearsOnTable<QS> + NonAggregate,
    Predicate::SqlType: BoolOrNullableBool,
{
    type Output = Self;

    fn or_filter(mut self, predicate: Predicate) -> Self::Output {
        self.where_clause = self.where_clause.or(predicate);
        self
    }
}

impl<'a, ST, DB, Predicate, GB> OrFilterDsl<Predicate>
    for BoxedCloneSelectStatement<'a, ST, NoFromClause, DB, GB>
where
    BoxedCloneWhereClause<'a, DB>: WhereOr<Predicate, Output = BoxedCloneWhereClause<'a, DB>>,
    Predicate: AppearsOnTable<NoFromClause> + NonAggregate,
    Predicate::SqlType: BoolOrNullableBool,
{
    type Output = Self;

    fn or_filter(mut self, predicate: Predicate) -> Self::Output {
        self.where_clause = self.where_clause.or(predicate);
        self
    }
}

impl<ST, QS, DB, GB> LimitDsl for BoxedCloneSelectStatement<'_, ST, QS, DB, GB>
where
    DB: Backend,
    LimitClause<AsExprOf<i64, BigInt>>: QueryFragment<DB>,
{
    type Output = Self;

    fn limit(mut self, limit: i64) -> Self::Output {
        self.limit_offset.limit = Some(Arc::new(LimitClause(limit.into_sql::<BigInt>())));
        self
    }
}

impl<ST, QS, DB, GB> OffsetDsl for BoxedCloneSelectStatement<'_, ST, QS, DB, GB>
where
    DB: Backend,
    OffsetClause<AsExprOf<i64, BigInt>>: QueryFragment<DB>,
{
    type Output = Self;

    fn offset(mut self, offset: i64) -> Self::Output {
        self.limit_offset.offset = Some(Arc::new(OffsetClause(offset.into_sql::<BigInt>())));
        self
    }
}

// no impls for `NoFromClause` here because order is not really supported there yet
impl<'a, ST, QS, DB, Order, GB> OrderDsl<Order>
    for BoxedCloneSelectStatement<'a, ST, FromClause<QS>, DB, GB>
where
    DB: Backend,
    QS: QuerySource,
    Order: QueryFragment<DB> + AppearsOnTable<QS> + Send + Sync + 'a,
{
    type Output = Self;

    fn order(mut self, order: Order) -> Self::Output {
        self.order = OrderClause(order).into();
        self
    }
}

impl<'a, ST, QS, DB, Order, GB> ThenOrderDsl<Order>
    for BoxedCloneSelectStatement<'a, ST, FromClause<QS>, DB, GB>
where
    DB: Backend + 'a,
    QS: QuerySource,
    Order: QueryFragment<DB> + AppearsOnTable<QS> + Send + Sync + 'a,
{
    type Output = Self;

    fn then_order_by(mut self, order: Order) -> Self::Output {
        self.order = match self.order {
            Some(old) => Some(Arc::new((old, order))),
            None => Some(Arc::new(order)),
        };
        self
    }
}

impl<ST, QS, DB, Rhs> JoinTo<Rhs> for BoxedCloneSelectStatement<'_, ST, FromClause<QS>, DB, ()>
where
    QS: JoinTo<Rhs> + QuerySource,
{
    type FromClause = <QS as JoinTo<Rhs>>::FromClause;
    type OnClause = QS::OnClause;

    fn join_target(rhs: Rhs) -> (Self::FromClause, Self::OnClause) {
        QS::join_target(rhs)
    }
}

impl<ST, QS, DB, GB> QueryDsl for BoxedCloneSelectStatement<'_, ST, QS, DB, GB> {}

impl<ST, QS, DB, GB> RunQueryDslSupport for BoxedCloneSelectStatement<'_, ST, QS, DB, GB> {}

impl<ST, QS, DB, T, GB> Insertable<T> for BoxedCloneSelectStatement<'_, ST, QS, DB, GB>
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

impl<ST, QS, DB, T, GB> Insertable<T> for &BoxedCloneSelectStatement<'_, ST, QS, DB, GB>
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

impl<'a, ST, QS, DB, GB> SelectNullableDsl for BoxedCloneSelectStatement<'a, ST, QS, DB, GB>
where
    ST: IntoNullable,
{
    type Output = BoxedCloneSelectStatement<'a, ST::Nullable, QS, DB>;

    fn nullable(self) -> Self::Output {
        BoxedCloneSelectStatement {
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
    for BoxedCloneSelectStatement<'a, ST, FromClause<QS>, DB, GB>
where
    QS: QuerySource,
    DB: Backend,
    GB: Expression,
    HavingClause<Predicate>: QueryFragment<DB> + Send + Sync + 'a,
    Predicate: AppearsOnTable<QS>,
    Predicate::SqlType: BoolOrNullableBool,
{
    type Output = Self;

    fn having(mut self, predicate: Predicate) -> Self::Output {
        self.having = Arc::new(HavingClause(predicate));
        self
    }
}

impl<ST, QS, DB, GB> CombineDsl for BoxedCloneSelectStatement<'_, ST, QS, DB, GB>
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
            assert_send(users::table.into_boxed_clone::<$backend>());
            assert_send(
                users::table
                    .filter(users::id.eq(10))
                    .into_boxed_clone::<$backend>(),
            );
        };};
    }

    #[diesel_test_helper::test]
    fn boxed_is_send() {
        #[cfg(feature = "postgres")]
        assert_boxed_query_send!(crate::pg::Pg);

        #[cfg(feature = "__sqlite-shared")]
        assert_boxed_query_send!(crate::sqlite::Sqlite);

        #[cfg(feature = "mysql")]
        assert_boxed_query_send!(crate::mysql::Mysql);

        #[cfg(feature = "mariadb")]
        assert_boxed_query_send!(crate::mariadb::Mariadb);
    }
}
