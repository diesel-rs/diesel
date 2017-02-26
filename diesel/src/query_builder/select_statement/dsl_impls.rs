use backend::Backend;
use expression::*;
use query_builder::distinct_clause::DistinctClause;
use query_builder::group_by_clause::*;
use query_builder::limit_clause::*;
use query_builder::offset_clause::*;
use query_builder::order_clause::*;
use query_builder::where_clause::*;
use query_builder::{AsQuery, Query, QueryFragment, SelectStatement};
use query_dsl::*;
use query_dsl::boxed_dsl::InternalBoxedDsl;
use super::BoxedSelectStatement;
use types::{self, Bool};

impl<S, F, D, W, O, L, Of, G, Selection, Type> SelectDsl<Selection, Type>
    for SelectStatement<S, F, D, W, O, L, Of, G> where
        Selection: Expression,
        SelectStatement<Selection, F, D, W, O, L, Of, G>: Query<SqlType=Type>,
{
    type Output = SelectStatement<Selection, F, D, W, O, L, Of, G>;

    fn select(self, selection: Selection) -> Self::Output {
        SelectStatement::new(
            selection,
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

impl<ST, S, F, D, W, O, L, Of, G> DistinctDsl
    for SelectStatement<S, F, D, W, O, L, Of, G> where
        SelectStatement<S, F, D, W, O, L, Of, G>: AsQuery<SqlType=ST>,
        SelectStatement<S, F, DistinctClause, W, O, L, Of, G>: AsQuery<SqlType=ST>,
{
    type Output = SelectStatement<S, F, DistinctClause, W, O, L, Of, G>;

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

impl<ST, S, F, D, W, O, L, Of, G, Predicate> FilterDsl<Predicate>
    for SelectStatement<S, F, D, W, O, L, Of, G> where
        SelectStatement<S, F, D, W, O, L, Of, G>: AsQuery<SqlType=ST>,
        SelectStatement<S, F, D, W::Output, O, L, Of, G>: Query<SqlType=ST>,
        Predicate: AppearsOnTable<F, SqlType=Bool> + NonAggregate,
        W: WhereAnd<Predicate>,
{
    type Output = SelectStatement<S, F, D, W::Output, O, L, Of, G>;

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

impl<ST, S, F, D, W, O, L, Of, G, Expr> OrderDsl<Expr>
    for SelectStatement<S, F, D, W, O, L, Of, G> where
        Expr: AppearsOnTable<F>,
        SelectStatement<S, F, D, W, O, L, Of, G>: AsQuery<SqlType=ST>,
        SelectStatement<S, F, D, W, OrderClause<Expr>, L, Of, G>: AsQuery<SqlType=ST>,
{
    type Output = SelectStatement<S, F, D, W, OrderClause<Expr>, L, Of, G>;

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

impl<ST, S, F, D, W, O, L, Of, G> LimitDsl
    for SelectStatement<S, F, D, W, O, L, Of, G> where
        SelectStatement<S, F, D, W, O, L, Of, G>: AsQuery<SqlType=ST>,
        SelectStatement<S, F, D, W, O, LimitClause<Limit>, Of, G>: Query<SqlType=ST>,
{
    type Output = SelectStatement<S, F, D, W, O, LimitClause<Limit>, Of, G>;

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

impl<ST, S, F, D, W, O, L, Of, G> OffsetDsl
    for SelectStatement<S, F, D, W, O, L, Of, G> where
        SelectStatement<S, F, D, W, O, L, Of, G>: AsQuery<SqlType=ST>,
        SelectStatement<S, F, D, W, O, L, OffsetClause<Offset>, G>: AsQuery<SqlType=ST>,
{
    type Output = SelectStatement<S, F, D, W, O, L, OffsetClause<Offset>, G>;

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

impl<S, F, D, W, O, L, Of, G, Expr> GroupByDsl<Expr>
    for SelectStatement<S, F, D, W, O, L, Of, G> where
        SelectStatement<S, F, D, W, O, L, Of, GroupByClause<Expr>>: Query,
        Expr: Expression,
{
    type Output = SelectStatement<S, F, D, W, O, L, Of, GroupByClause<Expr>>;

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

impl<'a, S, F, D, W, O, L, Of, G, DB> InternalBoxedDsl<'a, DB>
    for SelectStatement<S, F, D, W, O, L, Of, G> where
        DB: Backend,
        S: QueryFragment<DB> + SelectableExpression<F> + 'a,
        D: QueryFragment<DB> + 'a,
        W: Into<Option<Box<QueryFragment<DB> + 'a>>>,
        O: QueryFragment<DB> + 'a,
        L: QueryFragment<DB> + 'a,
        Of: QueryFragment<DB> + 'a,
{
    type Output = BoxedSelectStatement<'a, S::SqlTypeForSelect, F, DB>;

    fn internal_into_boxed(self) -> Self::Output {
        BoxedSelectStatement::new(
            Box::new(self.select),
            self.from,
            Box::new(self.distinct),
            self.where_clause.into(),
            Box::new(self.order),
            Box::new(self.limit),
            Box::new(self.offset),
        )
    }
}
