use associations::HasTable;
use backend::Backend;
use expression::*;
use query_builder::distinct_clause::*;
use query_builder::group_by_clause::*;
use query_builder::limit_clause::*;
use query_builder::offset_clause::*;
use query_builder::order_clause::*;
use query_builder::select_clause::*;
use query_builder::update_statement::*;
use query_builder::where_clause::*;
use query_builder::{AsQuery, Query, QueryFragment, SelectStatement};
use query_dsl::*;
use query_dsl::boxed_dsl::InternalBoxedDsl;
use query_source::QuerySource;
use query_source::joins::{Join, JoinOn, JoinTo};
use super::BoxedSelectStatement;
use types::{self, Bool};

impl<F, S, D, W, O, L, Of, G, Rhs, Kind, On> InternalJoinDsl<Rhs, Kind, On>
    for SelectStatement<F, S, D, W, O, L, Of, G> where
        SelectStatement<JoinOn<Join<F, Rhs, Kind>, On>, S, D, W, O, L, Of, G>: AsQuery,
{
    type Output = SelectStatement<JoinOn<Join<F, Rhs, Kind>, On>, S, D, W, O, L, Of, G>;

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
        )
    }
}

impl<F, S, D, W, O, L, Of, G, Selection, Type> SelectDsl<Selection>
    for SelectStatement<F, S, D, W, O, L, Of, G> where
        Selection: Expression<SqlType=Type>,
        SelectStatement<F, SelectClause<Selection>, D, W, O, L, Of, G>: Query<SqlType=Type>,
{
    type Output = SelectStatement<F, SelectClause<Selection>, D, W, O, L, Of, G>;

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
        )
    }
}

impl<ST, F, S, D, W, O, L, Of, G> DistinctDsl
    for SelectStatement<F, S, D, W, O, L, Of, G> where
        SelectStatement<F, S, D, W, O, L, Of, G>: AsQuery<SqlType=ST>,
        SelectStatement<F, S, DistinctClause, W, O, L, Of, G>: AsQuery<SqlType=ST>,
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
        )
    }
}

impl<ST, F, S, D, W, O, L, Of, G, Predicate> FilterDsl<Predicate>
    for SelectStatement<F, S, D, W, O, L, Of, G> where
        SelectStatement<F, S, D, W, O, L, Of, G>: AsQuery<SqlType=ST>,
        SelectStatement<F, S, D, W::Output, O, L, Of, G>: Query<SqlType=ST>,
        Predicate: AppearsOnTable<F, SqlType=Bool> + NonAggregate,
        W: WhereAnd<Predicate>,
{
    type Output = SelectStatement<F, S, D, W::Output, O, L, Of, G>;

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
        )
    }
}

use expression_methods::EqAll;
use helper_types::Filter;
use query_source::Table;

impl<F, S, D, W, O, L, Of, G, PK> FindDsl<PK>
    for SelectStatement<F, S, D, W, O, L, Of, G> where
        F: Table,
        F::PrimaryKey: EqAll<PK>,
        Self: FilterDsl<<F::PrimaryKey as EqAll<PK>>::Output>
{
    type Output = Filter<Self, <F::PrimaryKey as EqAll<PK>>::Output>;

    fn find(self, id: PK) -> Self::Output {
        let primary_key = self.from.primary_key();
        self.filter(primary_key.eq_all(id))
    }
}

impl<ST, F, S, D, W, O, L, Of, G, Expr> OrderDsl<Expr>
    for SelectStatement<F, S, D, W, O, L, Of, G> where
        Expr: AppearsOnTable<F>,
        SelectStatement<F, S, D, W, O, L, Of, G>: AsQuery<SqlType=ST>,
        SelectStatement<F, S, D, W, OrderClause<Expr>, L, Of, G>: AsQuery<SqlType=ST>,
{
    type Output = SelectStatement<F, S, D, W, OrderClause<Expr>, L, Of, G>;

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
        )
    }
}

#[doc(hidden)]
pub type Limit = <i64 as AsExpression<types::BigInt>>::Expression;

impl<ST, F, S, D, W, O, L, Of, G> LimitDsl
    for SelectStatement<F, S, D, W, O, L, Of, G> where
        SelectStatement<F, S, D, W, O, L, Of, G>: AsQuery<SqlType=ST>,
        SelectStatement<F, S, D, W, O, LimitClause<Limit>, Of, G>: Query<SqlType=ST>,
{
    type Output = SelectStatement<F, S, D, W, O, LimitClause<Limit>, Of, G>;

    fn limit(self, limit: i64) -> Self::Output {
        let limit_clause = LimitClause(AsExpression::<types::BigInt>::as_expression(limit));
        SelectStatement::new(
            self.select,
            self.from,
            self.distinct,
            self.where_clause,
            self.order,
            limit_clause,
            self.offset,
            self.group_by,
        )
    }
}

#[doc(hidden)]
pub type Offset = Limit;

impl<ST, F, S, D, W, O, L, Of, G> OffsetDsl
    for SelectStatement<F, S, D, W, O, L, Of, G> where
        SelectStatement<F, S, D, W, O, L, Of, G>: AsQuery<SqlType=ST>,
        SelectStatement<F, S, D, W, O, L, OffsetClause<Offset>, G>: AsQuery<SqlType=ST>,
{
    type Output = SelectStatement<F, S, D, W, O, L, OffsetClause<Offset>, G>;

    fn offset(self, offset: i64) -> Self::Output {
        let offset_clause = OffsetClause(AsExpression::<types::BigInt>::as_expression(offset));
        SelectStatement::new(
            self.select,
            self.from,
            self.distinct,
            self.where_clause,
            self.order,
            self.limit,
            offset_clause,
            self.group_by,
        )
    }
}

impl<F, S, D, W, O, L, Of, G, Expr> GroupByDsl<Expr>
    for SelectStatement<F, S, D, W, O, L, Of, G> where
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
        )
    }
}

impl<'a, F, S, D, W, O, L, Of, G, DB> InternalBoxedDsl<'a, DB>
    for SelectStatement<F, SelectClause<S>, D, W, O, L, Of, G> where
        DB: Backend,
        S: QueryFragment<DB> + SelectableExpression<F> + 'a,
        D: QueryFragment<DB> + 'a,
        W: Into<Option<Box<QueryFragment<DB> + 'a>>>,
        O: QueryFragment<DB> + 'a,
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
            Box::new(self.order),
            Box::new(self.limit),
            Box::new(self.offset),
            Box::new(self.group_by),
        )
    }
}

impl<'a, F, D, W, O, L, Of, G, DB> InternalBoxedDsl<'a, DB>
    for SelectStatement<F, DefaultSelectClause, D, W, O, L, Of, G> where
        DB: Backend,
        F: QuerySource,
        F::DefaultSelection: QueryFragment<DB> + 'a,
        D: QueryFragment<DB> + 'a,
        W: Into<Option<Box<QueryFragment<DB> + 'a>>>,
        O: QueryFragment<DB> + 'a,
        L: QueryFragment<DB> + 'a,
        Of: QueryFragment<DB> + 'a,
        G: QueryFragment<DB> + 'a,
{
    type Output = BoxedSelectStatement<
        'a,
        <F::DefaultSelection as Expression>::SqlType,
        F,
        DB,
    >;

    fn internal_into_boxed(self) -> Self::Output {
        BoxedSelectStatement::new(
            Box::new(self.from.default_selection()),
            self.from,
            Box::new(self.distinct),
            self.where_clause.into(),
            Box::new(self.order),
            Box::new(self.limit),
            Box::new(self.offset),
            Box::new(self.group_by),
        )
    }
}

impl<F, S, D, W, O, L, Of, G> HasTable for SelectStatement<F, S, D, W, O, L, Of, G> where
    F: HasTable,
{
    type Table = F::Table;

    fn table() -> Self::Table {
        F::table()
    }
}

impl<F, W> IntoUpdateTarget
    for SelectStatement<F, DefaultSelectClause, NoDistinctClause, W> where
        SelectStatement<F, DefaultSelectClause, NoDistinctClause, W>: HasTable,
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
impl<F, S, D, W, O, L, Of, G, Rhs> JoinTo<Rhs>
    for SelectStatement<F, S, D, W, O, L, Of, G> where
        F: JoinTo<Rhs>,
{
    type FromClause = F::FromClause;
    type OnClause = F::OnClause;

    fn join_target(rhs: Rhs) -> (Self::FromClause, Self::OnClause) {
        F::join_target(rhs)
    }
}
