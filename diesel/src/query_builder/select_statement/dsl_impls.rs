use backend::Backend;
use expression::*;
use expression::aliased::Aliased;
use query_builder::group_by_clause::*;
use query_builder::limit_clause::*;
use query_builder::offset_clause::*;
use query_builder::order_clause::*;
use query_builder::where_clause::*;
use query_builder::{Query, QueryFragment, SelectStatement};
use query_dsl::*;
use super::BoxedSelectStatement;
use types::{self, Bool};

impl<ST, S, F, W, O, L, Of, G, Selection, Type> SelectDsl<Selection, Type>
    for SelectStatement<ST, S, F, W, O, L, Of, G> where
    Selection: Expression,
    SelectStatement<Type, Selection, F, W, O, L, Of, G>: Query<SqlType=Type>,
{
    type Output = SelectStatement<Type, Selection, F, W, O, L, Of, G>;

    fn select(self, selection: Selection) -> Self::Output {
        SelectStatement::new(selection, self.from, self.where_clause, self.order,
            self.limit, self.offset, self.group_by)
    }
}

impl<ST, S, F, W, O, L, Of, G, Predicate> FilterDsl<Predicate>
    for SelectStatement<ST, S, F, W, O, L, Of, G> where
    Predicate: SelectableExpression<F, SqlType=Bool> + NonAggregate,
    W: WhereAnd<Predicate>,
    SelectStatement<ST, S, F, W::Output, O, L, Of, G>: Query,
{
    type Output = SelectStatement<ST, S, F, W::Output, O, L, Of, G>;

    fn filter(self, predicate: Predicate) -> Self::Output {
        SelectStatement::new(self.select, self.from, self.where_clause.and(predicate),
            self.order, self.limit, self.offset, self.group_by)
    }
}

impl<ST, S, F, W, O, L, Of, G, Expr> OrderDsl<Expr>
    for SelectStatement<ST, S, F, W, O, L, Of, G> where
    Expr: SelectableExpression<F>,
    SelectStatement<ST, S, F, W, OrderClause<Expr>, L, Of, G>: Query<SqlType=ST>,
{
    type Output = SelectStatement<ST, S, F, W, OrderClause<Expr>, L, Of, G>;

    fn order(self, expr: Expr) -> Self::Output {
        let order = OrderClause(expr);
        SelectStatement::new(self.select, self.from, self.where_clause, order,
            self.limit, self.offset, self.group_by)
    }
}

#[doc(hidden)]
pub type Limit = <i64 as AsExpression<types::BigInt>>::Expression;

impl<ST, S, F, W, O, L, Of, G> LimitDsl for SelectStatement<ST, S, F, W, O, L, Of, G> where
    SelectStatement<ST, S, F, W, O, LimitClause<Limit>, Of, G>: Query<SqlType=ST>,
{
    type Output = SelectStatement<ST, S, F, W, O, LimitClause<Limit>, Of, G>;

    fn limit(self, limit: i64) -> Self::Output {
        let limit_clause = LimitClause(AsExpression::<types::BigInt>::as_expression(limit));
        SelectStatement::new(self.select, self.from, self.where_clause,
            self.order, limit_clause, self.offset, self.group_by)
    }
}

#[doc(hidden)]
pub type Offset = Limit;

impl<ST, S, F, W, O, L, Of, G> OffsetDsl for SelectStatement<ST, S, F, W, O, L, Of, G> where
    SelectStatement<ST, S, F, W, O, L, OffsetClause<Offset>, G>: Query<SqlType=ST>,
{
    type Output = SelectStatement<ST, S, F, W, O, L, OffsetClause<Offset>, G>;

    fn offset(self, offset: i64) -> Self::Output {
        let offset_clause = OffsetClause(AsExpression::<types::BigInt>::as_expression(offset));
        SelectStatement::new(self.select, self.from, self.where_clause,
            self.order, self.limit, offset_clause, self.group_by)
    }
}

impl<'a, ST, S, F, W, O, L, Of, G, Expr> WithDsl<'a, Expr>
for SelectStatement<ST, S, F, W, O, L, Of, G> where
    SelectStatement<ST, S, WithQuerySource<'a, F, Expr>, W, O, L, Of, G>: Query,
{
    type Output = SelectStatement<ST, S, WithQuerySource<'a, F, Expr>, W, O, L, Of, G>;

    fn with(self, expr: Aliased<'a, Expr>) -> Self::Output {
        let source = WithQuerySource::new(self.from, expr);
        SelectStatement::new(self.select, source, self.where_clause,
            self.order, self.limit, self.offset, self.group_by)
    }
}

impl<ST, S, F, W, O, L, Of, G, Expr> GroupByDsl<Expr>
for SelectStatement<ST, S, F, W, O, L, Of, G> where
    SelectStatement<ST, S, F, W, O, L, Of, GroupByClause<Expr>>: Query,
    Expr: Expression,
{
    type Output = SelectStatement<ST, S, F, W, O, L, Of, GroupByClause<Expr>>;

    fn group_by(self, expr: Expr) -> Self::Output {
        let group_by = GroupByClause(expr);
        SelectStatement::new(self.select, self.from, self.where_clause,
            self.order, self.limit, self.offset, group_by)
    }
}

impl<ST, S, F, W, O, L, Of, G, DB> BoxedDsl<DB>
for SelectStatement<ST, S, F, W, O, L, Of, G> where
    DB: Backend,
    S: QueryFragment<DB> + 'static,
    W: QueryFragment<DB> + 'static,
    O: QueryFragment<DB> + 'static,
    L: QueryFragment<DB> + 'static,
    Of: QueryFragment<DB> + 'static,
{
    type Output = BoxedSelectStatement<ST, F, DB>;

    fn into_boxed(self) -> Self::Output {
        BoxedSelectStatement::new(
            Box::new(self.select),
            self.from,
            Box::new(self.where_clause),
            Box::new(self.order),
            Box::new(self.limit),
            Box::new(self.offset),
        )
    }
}
