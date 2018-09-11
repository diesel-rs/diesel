use super::BoxedSelectStatement;
use associations::HasTable;
use backend::Backend;
use dsl::AsExprOf;
use expression::nullable::Nullable;
use expression::*;
use insertable::Insertable;
use query_builder::distinct_clause::*;
use query_builder::group_by_clause::*;
use query_builder::insert_statement::InsertFromSelect;
use query_builder::limit_clause::*;
use query_builder::locking_clause::*;
use query_builder::offset_clause::*;
use query_builder::order_clause::*;
use query_builder::select_clause::*;
use query_builder::update_statement::*;
use query_builder::where_clause::*;
use query_builder::{AsQuery, Query, QueryFragment, SelectQuery, SelectStatement};
use query_dsl::boxed_dsl::BoxedDsl;
use query_dsl::methods::*;
use query_dsl::*;
use query_source::joins::{Join, JoinOn, JoinTo};
use query_source::QuerySource;
use sql_types::{BigInt, Bool};

impl<F, S, D, W, O, L, Of, G, LC, Rhs, Kind, On> InternalJoinDsl<Rhs, Kind, On>
    for SelectStatement<F, S, D, W, O, L, Of, G, LC>
where
    SelectStatement<JoinOn<Join<F, Rhs, Kind>, On>, S, D, W, O, L, Of, G, LC>: AsQuery,
{
    type Output = SelectStatement<JoinOn<Join<F, Rhs, Kind>, On>, S, D, W, O, L, Of, G, LC>;

    fn join(self, rhs: Rhs, kind: Kind, on: On) -> Self::Output {
        SelectStatement::new(
            self.select,
            Join::new(self.from, rhs, kind).on(on),
            self.distinct,
            self.where_clause,
            self.order,
            self.limit,
            self.offset,
            self.group_by,
            self.locking,
        )
    }
}

impl<F, S, D, W, O, L, Of, G, LC, Selection> SelectDsl<Selection>
    for SelectStatement<F, S, D, W, O, L, Of, G, LC>
where
    Selection: SelectableExpression<F>,
    SelectStatement<F, SelectClause<Selection>, D, W, O, L, Of, G, LC>: SelectQuery,
{
    type Output = SelectStatement<F, SelectClause<Selection>, D, W, O, L, Of, G, LC>;

    fn select(self, selection: Selection) -> Self::Output {
        SelectStatement::new(
            SelectClause(selection),
            self.from,
            self.distinct,
            self.where_clause,
            self.order,
            self.limit,
            self.offset,
            self.group_by,
            self.locking,
        )
    }
}

impl<ST, F, S, D, W, O, L, Of, G> DistinctDsl for SelectStatement<F, S, D, W, O, L, Of, G>
where
    Self: SelectQuery<SqlType = ST>,
    SelectStatement<F, S, DistinctClause, W, O, L, Of, G>: SelectQuery<SqlType = ST>,
{
    type Output = SelectStatement<F, S, DistinctClause, W, O, L, Of, G>;

    fn distinct(self) -> Self::Output {
        SelectStatement::new(
            self.select,
            self.from,
            DistinctClause,
            self.where_clause,
            self.order,
            self.limit,
            self.offset,
            self.group_by,
            self.locking,
        )
    }
}

impl<F, S, D, W, O, L, Of, G, LC, Predicate> FilterDsl<Predicate>
    for SelectStatement<F, S, D, W, O, L, Of, G, LC>
where
    Predicate: Expression<SqlType = Bool> + NonAggregate,
    W: WhereAnd<Predicate>,
{
    type Output = SelectStatement<F, S, D, W::Output, O, L, Of, G, LC>;

    fn filter(self, predicate: Predicate) -> Self::Output {
        SelectStatement::new(
            self.select,
            self.from,
            self.distinct,
            self.where_clause.and(predicate),
            self.order,
            self.limit,
            self.offset,
            self.group_by,
            self.locking,
        )
    }
}

impl<F, S, D, W, O, L, Of, G, LC, Predicate> OrFilterDsl<Predicate>
    for SelectStatement<F, S, D, W, O, L, Of, G, LC>
where
    Predicate: Expression<SqlType = Bool> + NonAggregate,
    W: WhereOr<Predicate>,
{
    type Output = SelectStatement<F, S, D, W::Output, O, L, Of, G, LC>;

    fn or_filter(self, predicate: Predicate) -> Self::Output {
        SelectStatement::new(
            self.select,
            self.from,
            self.distinct,
            self.where_clause.or(predicate),
            self.order,
            self.limit,
            self.offset,
            self.group_by,
            self.locking,
        )
    }
}

use dsl::Filter;
use expression_methods::EqAll;
use query_source::Table;

impl<F, S, D, W, O, L, Of, G, LC, PK> FindDsl<PK> for SelectStatement<F, S, D, W, O, L, Of, G, LC>
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

impl<ST, F, S, D, W, O, L, Of, G, LC, Expr> OrderDsl<Expr>
    for SelectStatement<F, S, D, W, O, L, Of, G, LC>
where
    Expr: AppearsOnTable<F>,
    Self: SelectQuery<SqlType = ST>,
    SelectStatement<F, S, D, W, OrderClause<Expr>, L, Of, G, LC>: SelectQuery<SqlType = ST>,
{
    type Output = SelectStatement<F, S, D, W, OrderClause<Expr>, L, Of, G, LC>;

    fn order(self, expr: Expr) -> Self::Output {
        let order = OrderClause(expr);
        SelectStatement::new(
            self.select,
            self.from,
            self.distinct,
            self.where_clause,
            order,
            self.limit,
            self.offset,
            self.group_by,
            self.locking,
        )
    }
}

impl<F, S, D, W, O, L, Of, G, LC, Expr> ThenOrderDsl<Expr>
    for SelectStatement<F, S, D, W, OrderClause<O>, L, Of, G, LC>
where
    Expr: AppearsOnTable<F>,
{
    type Output = SelectStatement<F, S, D, W, OrderClause<(O, Expr)>, L, Of, G, LC>;

    fn then_order_by(self, expr: Expr) -> Self::Output {
        SelectStatement::new(
            self.select,
            self.from,
            self.distinct,
            self.where_clause,
            OrderClause((self.order.0, expr)),
            self.limit,
            self.offset,
            self.group_by,
            self.locking,
        )
    }
}

impl<F, S, D, W, L, Of, G, LC, Expr> ThenOrderDsl<Expr>
    for SelectStatement<F, S, D, W, NoOrderClause, L, Of, G, LC>
where
    Expr: Expression,
    Self: OrderDsl<Expr>,
{
    type Output = ::dsl::Order<Self, Expr>;

    fn then_order_by(self, expr: Expr) -> Self::Output {
        self.order_by(expr)
    }
}

#[doc(hidden)]
pub type Limit = AsExprOf<i64, BigInt>;

impl<ST, F, S, D, W, O, L, Of, G, LC> LimitDsl for SelectStatement<F, S, D, W, O, L, Of, G, LC>
where
    Self: SelectQuery<SqlType = ST>,
    SelectStatement<F, S, D, W, O, LimitClause<Limit>, Of, G, LC>: SelectQuery<SqlType = ST>,
{
    type Output = SelectStatement<F, S, D, W, O, LimitClause<Limit>, Of, G, LC>;

    fn limit(self, limit: i64) -> Self::Output {
        let limit_clause = LimitClause(limit.into_sql::<BigInt>());
        SelectStatement::new(
            self.select,
            self.from,
            self.distinct,
            self.where_clause,
            self.order,
            limit_clause,
            self.offset,
            self.group_by,
            self.locking,
        )
    }
}

#[doc(hidden)]
pub type Offset = Limit;

impl<ST, F, S, D, W, O, L, Of, G, LC> OffsetDsl for SelectStatement<F, S, D, W, O, L, Of, G, LC>
where
    Self: SelectQuery<SqlType = ST>,
    SelectStatement<F, S, D, W, O, L, OffsetClause<Offset>, G, LC>: SelectQuery<SqlType = ST>,
{
    type Output = SelectStatement<F, S, D, W, O, L, OffsetClause<Offset>, G, LC>;

    fn offset(self, offset: i64) -> Self::Output {
        let offset_clause = OffsetClause(offset.into_sql::<BigInt>());
        SelectStatement::new(
            self.select,
            self.from,
            self.distinct,
            self.where_clause,
            self.order,
            self.limit,
            offset_clause,
            self.group_by,
            self.locking,
        )
    }
}

impl<F, S, D, W, O, L, Of, G, Expr> GroupByDsl<Expr> for SelectStatement<F, S, D, W, O, L, Of, G>
where
    SelectStatement<F, S, D, W, O, L, Of, GroupByClause<Expr>>: Query,
    Expr: Expression,
{
    type Output = SelectStatement<F, S, D, W, O, L, Of, GroupByClause<Expr>>;

    fn group_by(self, expr: Expr) -> Self::Output {
        let group_by = GroupByClause(expr);
        SelectStatement::new(
            self.select,
            self.from,
            self.distinct,
            self.where_clause,
            self.order,
            self.limit,
            self.offset,
            group_by,
            self.locking,
        )
    }
}

impl<F, S, W, O, L, Of, Lock> LockingDsl<Lock>
    for SelectStatement<F, S, NoDistinctClause, W, O, L, Of>
{
    type Output = SelectStatement<
        F,
        S,
        NoDistinctClause,
        W,
        O,
        L,
        Of,
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
            self.limit,
            self.offset,
            self.group_by,
            LockingClause::new(lock, NoModifier),
        )
    }
}

impl<F, S, D, W, O, L, Of, G, LC, LM, Modifier> ModifyLockDsl<Modifier>
    for SelectStatement<F, S, D, W, O, L, Of, G, LockingClause<LC, LM>>
{
    type Output = SelectStatement<F, S, D, W, O, L, Of, G, LockingClause<LC, Modifier>>;

    fn modify_lock(self, modifier: Modifier) -> Self::Output {
        SelectStatement::new(
            self.select,
            self.from,
            self.distinct,
            self.where_clause,
            self.order,
            self.limit,
            self.offset,
            self.group_by,
            LockingClause::new(self.locking.lock_mode, modifier),
        )
    }
}

impl<'a, F, S, D, W, O, L, Of, G, DB> BoxedDsl<'a, DB>
    for SelectStatement<F, SelectClause<S>, D, W, O, L, Of, G>
where
    Self: AsQuery,
    DB: Backend,
    S: QueryFragment<DB> + SelectableExpression<F> + 'a,
    D: QueryFragment<DB> + 'a,
    W: Into<BoxedWhereClause<'a, DB>>,
    O: Into<Option<Box<QueryFragment<DB> + 'a>>>,
    L: QueryFragment<DB> + 'a,
    Of: QueryFragment<DB> + 'a,
    G: QueryFragment<DB> + 'a,
{
    type Output = BoxedSelectStatement<'a, S::SqlType, F, DB>;

    fn internal_into_boxed(self) -> Self::Output {
        BoxedSelectStatement::new(
            Box::new(self.select.0),
            self.from,
            Box::new(self.distinct),
            self.where_clause.into(),
            self.order.into(),
            Box::new(self.limit),
            Box::new(self.offset),
            Box::new(self.group_by),
        )
    }
}

impl<'a, F, D, W, O, L, Of, G, DB> BoxedDsl<'a, DB>
    for SelectStatement<F, DefaultSelectClause, D, W, O, L, Of, G>
where
    Self: AsQuery,
    DB: Backend,
    F: QuerySource,
    F::DefaultSelection: QueryFragment<DB> + 'a,
    D: QueryFragment<DB> + 'a,
    W: Into<BoxedWhereClause<'a, DB>>,
    O: Into<Option<Box<QueryFragment<DB> + 'a>>>,
    L: QueryFragment<DB> + 'a,
    Of: QueryFragment<DB> + 'a,
    G: QueryFragment<DB> + 'a,
{
    type Output = BoxedSelectStatement<'a, <F::DefaultSelection as Expression>::SqlType, F, DB>;
    fn internal_into_boxed(self) -> Self::Output {
        BoxedSelectStatement::new(
            Box::new(self.from.default_selection()),
            self.from,
            Box::new(self.distinct),
            self.where_clause.into(),
            self.order.into(),
            Box::new(self.limit),
            Box::new(self.offset),
            Box::new(self.group_by),
        )
    }
}

impl<F, S, D, W, O, L, Of, G, LC> HasTable for SelectStatement<F, S, D, W, O, L, Of, G, LC>
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
impl<F, S, D, W, O, L, Of, G, LC, Rhs> JoinTo<Rhs> for SelectStatement<F, S, D, W, O, L, Of, G, LC>
where
    F: JoinTo<Rhs>,
{
    type FromClause = F::FromClause;
    type OnClause = F::OnClause;

    fn join_target(rhs: Rhs) -> (Self::FromClause, Self::OnClause) {
        F::join_target(rhs)
    }
}

impl<F, S, D, W, O, L, Of, G, LC> QueryDsl for SelectStatement<F, S, D, W, O, L, Of, G, LC> {}

impl<F, S, D, W, O, L, Of, G, LC, Conn> RunQueryDsl<Conn>
    for SelectStatement<F, S, D, W, O, L, Of, G, LC>
{
}

impl<F, S, D, W, O, L, Of, G, LC, Tab> Insertable<Tab>
    for SelectStatement<F, S, D, W, O, L, Of, G, LC>
where
    Tab: Table,
    Self: Query,
{
    type Values = InsertFromSelect<Self, Tab::AllColumns>;

    fn values(self) -> Self::Values {
        InsertFromSelect::new(self)
    }
}

impl<'a, F, S, D, W, O, L, Of, G, LC, Tab> Insertable<Tab>
    for &'a SelectStatement<F, S, D, W, O, L, Of, G, LC>
where
    Tab: Table,
    Self: Query,
{
    type Values = InsertFromSelect<Self, Tab::AllColumns>;

    fn values(self) -> Self::Values {
        InsertFromSelect::new(self)
    }
}

impl<'a, F, S, D, W, O, L, Of, G> SelectNullableDsl
    for SelectStatement<F, SelectClause<S>, D, W, O, L, Of, G>
{
    type Output = SelectStatement<F, SelectClause<Nullable<S>>, D, W, O, L, Of, G>;

    fn nullable(self) -> Self::Output {
        SelectStatement::new(
            SelectClause(Nullable::new(self.select.0)),
            self.from,
            self.distinct,
            self.where_clause,
            self.order,
            self.limit,
            self.offset,
            self.group_by,
            self.locking,
        )
    }
}

impl<'a, F, D, W, O, L, Of, G> SelectNullableDsl
    for SelectStatement<F, DefaultSelectClause, D, W, O, L, Of, G>
where
    F: QuerySource,
{
    type Output =
        SelectStatement<F, SelectClause<Nullable<F::DefaultSelection>>, D, W, O, L, Of, G>;

    fn nullable(self) -> Self::Output {
        SelectStatement::new(
            SelectClause(Nullable::new(self.from.default_selection())),
            self.from,
            self.distinct,
            self.where_clause,
            self.order,
            self.limit,
            self.offset,
            self.group_by,
            self.locking,
        )
    }
}
