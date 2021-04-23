use super::BoxedSelectStatement;
use crate::associations::HasTable;
use crate::backend::Backend;
use crate::dsl::AsExprOf;
use crate::expression::nullable::Nullable;
use crate::expression::*;
use crate::insertable::Insertable;
use crate::query_builder::combination_clause::*;
use crate::query_builder::distinct_clause::*;
use crate::query_builder::group_by_clause::*;
use crate::query_builder::insert_statement::InsertFromSelect;
use crate::query_builder::limit_clause::*;
use crate::query_builder::limit_offset_clause::{BoxedLimitOffsetClause, LimitOffsetClause};
use crate::query_builder::locking_clause::*;
use crate::query_builder::offset_clause::*;
use crate::query_builder::order_clause::*;
use crate::query_builder::select_clause::*;
use crate::query_builder::update_statement::*;
use crate::query_builder::where_clause::*;
use crate::query_builder::{
    AsQuery, IntoBoxedClause, Query, QueryFragment, SelectQuery, SelectStatement,
};
use crate::query_dsl::boxed_dsl::BoxedDsl;
use crate::query_dsl::methods::*;
use crate::query_dsl::order_dsl::ValidOrderingForDistinct;
use crate::query_dsl::*;
use crate::query_source::joins::{Join, JoinOn, JoinTo};
use crate::query_source::QuerySource;
use crate::sql_types::{BigInt, BoolOrNullableBool};

impl<F, S, D, W, O, LOf, G, LC, H, Rhs, Kind, On> InternalJoinDsl<Rhs, Kind, On>
    for SelectStatement<F, S, D, W, O, LOf, G, LC, H>
where
    SelectStatement<JoinOn<Join<F, Rhs, Kind>, On>, S, D, W, O, LOf, G, LC, H>: AsQuery,
{
    type Output = SelectStatement<JoinOn<Join<F, Rhs, Kind>, On>, S, D, W, O, LOf, G, LC, H>;

    fn join(self, rhs: Rhs, kind: Kind, on: On) -> Self::Output {
        SelectStatement::new(
            self.select,
            Join::new(self.from, rhs, kind).on(on),
            self.distinct,
            self.where_clause,
            self.order,
            self.limit_offset,
            self.group_by,
            self.locking,
            self.having,
        )
    }
}

impl<F, S, D, W, O, LOf, G, LC, H, Selection> SelectDsl<Selection>
    for SelectStatement<F, S, D, W, O, LOf, G, LC, H>
where
    G: ValidGroupByClause,
    Selection: SelectableExpression<F> + ValidGrouping<G::Expressions>,
    SelectStatement<F, SelectClause<Selection>, D, W, O, LOf, G, LC, H>: SelectQuery,
{
    type Output = SelectStatement<F, SelectClause<Selection>, D, W, O, LOf, G, LC, H>;

    fn select(self, selection: Selection) -> Self::Output {
        SelectStatement::new(
            SelectClause(selection),
            self.from,
            self.distinct,
            self.where_clause,
            self.order,
            self.limit_offset,
            self.group_by,
            self.locking,
            self.having,
        )
    }
}

impl<ST, F, S, D, W, O, LOf, G, H> DistinctDsl
    for SelectStatement<F, S, D, W, O, LOf, G, NoLockingClause, H>
where
    Self: SelectQuery<SqlType = ST>,
    SelectStatement<F, S, DistinctClause, W, O, LOf, G, NoLockingClause, H>:
        SelectQuery<SqlType = ST>,
{
    type Output = SelectStatement<F, S, DistinctClause, W, O, LOf, G, NoLockingClause, H>;

    fn distinct(self) -> Self::Output {
        SelectStatement::new(
            self.select,
            self.from,
            DistinctClause,
            self.where_clause,
            self.order,
            self.limit_offset,
            self.group_by,
            self.locking,
            self.having,
        )
    }
}

impl<F, S, D, W, O, LOf, G, LC, H, Predicate> FilterDsl<Predicate>
    for SelectStatement<F, S, D, W, O, LOf, G, LC, H>
where
    Predicate: Expression + NonAggregate,
    Predicate::SqlType: BoolOrNullableBool,
    W: WhereAnd<Predicate>,
{
    type Output = SelectStatement<F, S, D, W::Output, O, LOf, G, LC, H>;

    fn filter(self, predicate: Predicate) -> Self::Output {
        SelectStatement::new(
            self.select,
            self.from,
            self.distinct,
            self.where_clause.and(predicate),
            self.order,
            self.limit_offset,
            self.group_by,
            self.locking,
            self.having,
        )
    }
}

impl<F, S, D, W, O, LOf, G, LC, H, Predicate> OrFilterDsl<Predicate>
    for SelectStatement<F, S, D, W, O, LOf, G, LC, H>
where
    Predicate: Expression + NonAggregate,
    Predicate::SqlType: BoolOrNullableBool,
    W: WhereOr<Predicate>,
{
    type Output = SelectStatement<F, S, D, W::Output, O, LOf, G, LC, H>;

    fn or_filter(self, predicate: Predicate) -> Self::Output {
        SelectStatement::new(
            self.select,
            self.from,
            self.distinct,
            self.where_clause.or(predicate),
            self.order,
            self.limit_offset,
            self.group_by,
            self.locking,
            self.having,
        )
    }
}

use crate::dsl::Filter;
use crate::expression_methods::EqAll;
use crate::query_builder::having_clause::HavingClause;
use crate::query_source::Table;

impl<F, S, D, W, O, LOf, G, LC, H, PK> FindDsl<PK> for SelectStatement<F, S, D, W, O, LOf, G, LC, H>
where
    F: Table,
    F::PrimaryKey: EqAll<PK>,
    Self: FilterDsl<<F::PrimaryKey as EqAll<PK>>::Output>,
{
    type Output = Filter<Self, <F::PrimaryKey as EqAll<PK>>::Output>;

    fn find(self, id: PK) -> Self::Output {
        let primary_key = self.from.primary_key();
        FilterDsl::filter(self, primary_key.eq_all(id))
    }
}

impl<ST, F, S, D, W, O, LOf, G, LC, H, Expr> OrderDsl<Expr>
    for SelectStatement<F, S, D, W, O, LOf, G, LC, H>
where
    Expr: AppearsOnTable<F> + ValidOrderingForDistinct<D>,
    Self: SelectQuery<SqlType = ST>,
    SelectStatement<F, S, D, W, OrderClause<Expr>, LOf, G, LC, H>: SelectQuery<SqlType = ST>,
{
    type Output = SelectStatement<F, S, D, W, OrderClause<Expr>, LOf, G, LC, H>;

    fn order(self, expr: Expr) -> Self::Output {
        let order = OrderClause(expr);
        SelectStatement::new(
            self.select,
            self.from,
            self.distinct,
            self.where_clause,
            order,
            self.limit_offset,
            self.group_by,
            self.locking,
            self.having,
        )
    }
}

impl<F, S, D, W, O, LOf, G, LC, H, Expr> ThenOrderDsl<Expr>
    for SelectStatement<F, S, D, W, OrderClause<O>, LOf, G, LC, H>
where
    Expr: AppearsOnTable<F>,
{
    type Output = SelectStatement<F, S, D, W, OrderClause<(O, Expr)>, LOf, G, LC, H>;

    fn then_order_by(self, expr: Expr) -> Self::Output {
        SelectStatement::new(
            self.select,
            self.from,
            self.distinct,
            self.where_clause,
            OrderClause((self.order.0, expr)),
            self.limit_offset,
            self.group_by,
            self.locking,
            self.having,
        )
    }
}

impl<F, S, D, W, LOf, G, LC, Expr> ThenOrderDsl<Expr>
    for SelectStatement<F, S, D, W, NoOrderClause, LOf, G, LC>
where
    Expr: Expression,
    Self: OrderDsl<Expr>,
{
    type Output = crate::dsl::Order<Self, Expr>;

    fn then_order_by(self, expr: Expr) -> Self::Output {
        self.order_by(expr)
    }
}

#[doc(hidden)]
pub type Limit = AsExprOf<i64, BigInt>;

impl<ST, F, S, D, W, O, L, Of, G, LC, H> LimitDsl
    for SelectStatement<F, S, D, W, O, LimitOffsetClause<L, Of>, G, LC, H>
where
    Self: SelectQuery<SqlType = ST>,
    SelectStatement<F, S, D, W, O, LimitOffsetClause<LimitClause<Limit>, Of>, G, LC, H>:
        SelectQuery<SqlType = ST>,
{
    type Output =
        SelectStatement<F, S, D, W, O, LimitOffsetClause<LimitClause<Limit>, Of>, G, LC, H>;

    fn limit(self, limit: i64) -> Self::Output {
        let limit_clause = LimitClause(limit.into_sql::<BigInt>());
        SelectStatement::new(
            self.select,
            self.from,
            self.distinct,
            self.where_clause,
            self.order,
            LimitOffsetClause {
                limit_clause,
                offset_clause: self.limit_offset.offset_clause,
            },
            self.group_by,
            self.locking,
            self.having,
        )
    }
}

#[doc(hidden)]
pub type Offset = Limit;

impl<ST, F, S, D, W, O, L, Of, G, LC, H> OffsetDsl
    for SelectStatement<F, S, D, W, O, LimitOffsetClause<L, Of>, G, LC, H>
where
    Self: SelectQuery<SqlType = ST>,
    SelectStatement<F, S, D, W, O, LimitOffsetClause<L, OffsetClause<Offset>>, G, LC, H>:
        SelectQuery<SqlType = ST>,
{
    type Output =
        SelectStatement<F, S, D, W, O, LimitOffsetClause<L, OffsetClause<Offset>>, G, LC, H>;

    fn offset(self, offset: i64) -> Self::Output {
        let offset_clause = OffsetClause(offset.into_sql::<BigInt>());
        SelectStatement::new(
            self.select,
            self.from,
            self.distinct,
            self.where_clause,
            self.order,
            LimitOffsetClause {
                limit_clause: self.limit_offset.limit_clause,
                offset_clause,
            },
            self.group_by,
            self.locking,
            self.having,
        )
    }
}

impl<F, S, D, W, O, LOf, G, H, Expr> GroupByDsl<Expr>
    for SelectStatement<F, S, D, W, O, LOf, G, NoLockingClause, H>
where
    SelectStatement<F, S, D, W, O, LOf, GroupByClause<Expr>, NoLockingClause, H>: SelectQuery,
    Expr: Expression,
{
    type Output = SelectStatement<F, S, D, W, O, LOf, GroupByClause<Expr>, NoLockingClause, H>;

    fn group_by(self, expr: Expr) -> Self::Output {
        let group_by = GroupByClause(expr);
        SelectStatement::new(
            self.select,
            self.from,
            self.distinct,
            self.where_clause,
            self.order,
            self.limit_offset,
            group_by,
            self.locking,
            self.having,
        )
    }
}

impl<F, S, W, O, LOf, Lock> LockingDsl<Lock>
    for SelectStatement<F, S, NoDistinctClause, W, O, LOf>
{
    type Output = SelectStatement<
        F,
        S,
        NoDistinctClause,
        W,
        O,
        LOf,
        NoGroupByClause,
        LockingClause<Lock, NoModifier>,
    >;

    fn with_lock(self, lock: Lock) -> Self::Output {
        SelectStatement::new(
            self.select,
            self.from,
            self.distinct,
            self.where_clause,
            self.order,
            self.limit_offset,
            self.group_by,
            LockingClause::new(lock, NoModifier),
            self.having,
        )
    }
}

impl<F, S, D, W, O, LOf, G, LC, LM, H, Modifier> ModifyLockDsl<Modifier>
    for SelectStatement<F, S, D, W, O, LOf, G, LockingClause<LC, LM>, H>
{
    type Output = SelectStatement<F, S, D, W, O, LOf, G, LockingClause<LC, Modifier>, H>;

    fn modify_lock(self, modifier: Modifier) -> Self::Output {
        SelectStatement::new(
            self.select,
            self.from,
            self.distinct,
            self.where_clause,
            self.order,
            self.limit_offset,
            self.group_by,
            LockingClause::new(self.locking.lock_mode, modifier),
            self.having,
        )
    }
}

impl<'a, F, S, D, W, O, LOf, G, H, DB> BoxedDsl<'a, DB>
    for SelectStatement<F, S, D, W, O, LOf, G, NoLockingClause, H>
where
    Self: AsQuery,
    DB: Backend,
    S: IntoBoxedSelectClause<'a, DB, F> + SelectClauseExpression<F>,
    S::Selection: ValidGrouping<G::Expressions>,
    D: QueryFragment<DB> + Send + 'a,
    W: Into<BoxedWhereClause<'a, DB>>,
    O: Into<Option<Box<dyn QueryFragment<DB> + Send + 'a>>>,
    LOf: IntoBoxedClause<'a, DB, BoxedClause = BoxedLimitOffsetClause<'a, DB>>,
    G: ValidGroupByClause + QueryFragment<DB> + Send + 'a,
    H: QueryFragment<DB> + Send + 'a,
{
    type Output = BoxedSelectStatement<'a, S::SqlType, F, DB, G::Expressions>;

    fn internal_into_boxed(self) -> Self::Output {
        BoxedSelectStatement::new(
            self.select,
            self.from,
            Box::new(self.distinct),
            self.where_clause.into(),
            self.order.into(),
            self.limit_offset.into_boxed(),
            self.group_by,
            Box::new(self.having),
        )
    }
}

impl<F, S, D, W, O, LOf, G, LC, H> HasTable for SelectStatement<F, S, D, W, O, LOf, G, LC, H>
where
    F: HasTable,
{
    type Table = F::Table;

    fn table() -> Self::Table {
        F::table()
    }
}

impl<F, W> IntoUpdateTarget for SelectStatement<F, DefaultSelectClause, NoDistinctClause, W>
where
    SelectStatement<F, DefaultSelectClause, NoDistinctClause, W>: HasTable,
    W: ValidWhereClause<F>,
{
    type WhereClause = W;

    fn into_update_target(self) -> UpdateTarget<Self::Table, Self::WhereClause> {
        UpdateTarget {
            table: Self::table(),
            where_clause: self.where_clause,
        }
    }
}

// FIXME: Should we disable joining when `.group_by` has been called? Are there
// any other query methods where a join no longer has the same semantics as
// joining on just the table?
impl<F, S, D, W, O, LOf, G, LC, H, Rhs> JoinTo<Rhs>
    for SelectStatement<F, S, D, W, O, LOf, G, LC, H>
where
    F: JoinTo<Rhs>,
{
    type FromClause = F::FromClause;
    type OnClause = F::OnClause;

    fn join_target(rhs: Rhs) -> (Self::FromClause, Self::OnClause) {
        F::join_target(rhs)
    }
}

impl<F, S, D, W, O, LOf, G, LC, H> QueryDsl for SelectStatement<F, S, D, W, O, LOf, G, LC, H> {}

impl<F, S, D, W, O, LOf, G, LC, H, Conn> RunQueryDsl<Conn>
    for SelectStatement<F, S, D, W, O, LOf, G, LC, H>
{
}

impl<F, S, D, W, O, LOf, G, LC, H, Tab> Insertable<Tab>
    for SelectStatement<F, S, D, W, O, LOf, G, LC, H>
where
    Tab: Table,
    Self: Query,
    <Tab::AllColumns as ValidGrouping<()>>::IsAggregate:
        MixedAggregates<is_aggregate::No, Output = is_aggregate::No>,
{
    type Values = InsertFromSelect<Self, Tab::AllColumns>;

    fn values(self) -> Self::Values {
        InsertFromSelect::new(self)
    }
}

impl<'a, F, S, D, W, O, LOf, G, LC, H, Tab> Insertable<Tab>
    for &'a SelectStatement<F, S, D, W, O, LOf, G, LC, H>
where
    Tab: Table,
    Self: Query,
    <Tab::AllColumns as ValidGrouping<()>>::IsAggregate:
        MixedAggregates<is_aggregate::No, Output = is_aggregate::No>,
{
    type Values = InsertFromSelect<Self, Tab::AllColumns>;

    fn values(self) -> Self::Values {
        InsertFromSelect::new(self)
    }
}

impl<'a, F, S, D, W, O, LOf, G, H> SelectNullableDsl
    for SelectStatement<F, SelectClause<S>, D, W, O, LOf, G, NoLockingClause, H>
{
    type Output =
        SelectStatement<F, SelectClause<Nullable<S>>, D, W, O, LOf, G, NoLockingClause, H>;

    fn nullable(self) -> Self::Output {
        SelectStatement::new(
            SelectClause(Nullable::new(self.select.0)),
            self.from,
            self.distinct,
            self.where_clause,
            self.order,
            self.limit_offset,
            self.group_by,
            self.locking,
            self.having,
        )
    }
}

impl<'a, F, D, W, O, LOf, G, H> SelectNullableDsl
    for SelectStatement<F, DefaultSelectClause, D, W, O, LOf, G, NoLockingClause, H>
where
    F: QuerySource,
{
    type Output = SelectStatement<
        F,
        SelectClause<Nullable<F::DefaultSelection>>,
        D,
        W,
        O,
        LOf,
        G,
        NoLockingClause,
        H,
    >;

    fn nullable(self) -> Self::Output {
        SelectStatement::new(
            SelectClause(Nullable::new(self.from.default_selection())),
            self.from,
            self.distinct,
            self.where_clause,
            self.order,
            self.limit_offset,
            self.group_by,
            self.locking,
            self.having,
        )
    }
}

impl<F, S, D, W, O, LOf, G, H, Predicate> HavingDsl<Predicate>
    for SelectStatement<F, S, D, W, O, LOf, G, NoLockingClause, H>
where
    Predicate: Expression,
    Predicate::SqlType: BoolOrNullableBool,
    G: ValidGroupByClause,
{
    type Output = SelectStatement<F, S, D, W, O, LOf, G, NoLockingClause, HavingClause<Predicate>>;

    fn having(self, predicate: Predicate) -> Self::Output {
        SelectStatement::new(
            self.select,
            self.from,
            self.distinct,
            self.where_clause,
            self.order,
            self.limit_offset,
            self.group_by,
            self.locking,
            HavingClause(predicate),
        )
    }
}

impl<F, S, D, W, O, LOf, G, LC, H> CombineDsl for SelectStatement<F, S, D, W, O, LOf, G, LC, H>
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
