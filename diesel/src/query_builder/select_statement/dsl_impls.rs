use expression::*;
use expression::aliased::Aliased;
use query_builder::{Query, SelectStatement};
use query_builder::limit_clause::*;
use query_builder::offset_clause::*;
use query_builder::order_clause::*;
use query_builder::where_clause::*;
use query_dsl::*;
use types::{self, Bool, NativeSqlType};

impl<ST, S, F, W, O, L, Of, Selection, Type> SelectDsl<Selection, Type>
    for SelectStatement<ST, S, F, W, O, L, Of> where
    Selection: Expression,
    SelectStatement<Type, Selection, F, W, O, L, Of>: Query<SqlType=Type>,
    Type: NativeSqlType,
{
    type Output = SelectStatement<Type, Selection, F, W, O, L, Of>;

    fn select(self, selection: Selection) -> Self::Output {
        SelectStatement::new(selection, self.from, self.where_clause, self.order,
            self.limit, self.offset)
    }
}

impl<ST, S, F, W, O, L, Of, Predicate> FilterDsl<Predicate>
    for SelectStatement<ST, S, F, W, O, L, Of> where
    Predicate: SelectableExpression<F, SqlType=Bool> + NonAggregate,
    W: WhereAnd<Predicate>,
    SelectStatement<ST, S, F, W::Output, O, L, Of>: Query,
{
    type Output = SelectStatement<ST, S, F, W::Output, O, L, Of>;

    fn filter(self, predicate: Predicate) -> Self::Output {
        SelectStatement::new(self.select, self.from, self.where_clause.and(predicate),
            self.order, self.limit, self.offset)
    }
}

impl<ST, S, F, W, O, L, Of, Expr> OrderDsl<Expr>
    for SelectStatement<ST, S, F, W, O, L, Of> where
    ST: NativeSqlType,
    Expr: SelectableExpression<F>,
    SelectStatement<ST, S, F, W, OrderClause<Expr>, L, Of>: Query<SqlType=ST>,
{
    type Output = SelectStatement<ST, S, F, W, OrderClause<Expr>, L, Of>;

    fn order(self, expr: Expr) -> Self::Output {
        let order = OrderClause(expr);
        SelectStatement::new(self.select, self.from, self.where_clause, order,
            self.limit, self.offset)
    }
}

#[doc(hidden)]
pub type Limit = <i64 as AsExpression<types::BigInt>>::Expression;

impl<ST, S, F, W, O, L, Of> LimitDsl for SelectStatement<ST, S, F, W, O, L, Of> where
    ST: NativeSqlType,
    SelectStatement<ST, S, F, W, O, LimitClause<Limit>, Of>: Query<SqlType=ST>,
{
    type Output = SelectStatement<ST, S, F, W, O, LimitClause<Limit>, Of>;

    fn limit(self, limit: i64) -> Self::Output {
        let limit_clause = LimitClause(AsExpression::<types::BigInt>::as_expression(limit));
        SelectStatement::new(self.select, self.from, self.where_clause,
            self.order, limit_clause, self.offset)
    }
}

#[doc(hidden)]
pub type Offset = Limit;

impl<ST, S, F, W, O, L, Of> OffsetDsl for SelectStatement<ST, S, F, W, O, L, Of> where
    ST: NativeSqlType,
    SelectStatement<ST, S, F, W, O, L, OffsetClause<Offset>>: Query<SqlType=ST>,
{
    type Output = SelectStatement<ST, S, F, W, O, L, OffsetClause<Offset>>;

    fn offset(self, offset: i64) -> Self::Output {
        let offset_clause = OffsetClause(AsExpression::<types::BigInt>::as_expression(offset));
        SelectStatement::new(self.select, self.from, self.where_clause,
            self.order, self.limit, offset_clause)
    }
}

impl<'a, ST, S, F, W, O, L, Of, Expr> WithDsl<'a, Expr>
for SelectStatement<ST, S, F, W, O, L, Of> where
    SelectStatement<ST, S, WithQuerySource<'a, F, Expr>, W, O, L, Of>: Query,
{
    type Output = SelectStatement<ST, S, WithQuerySource<'a, F, Expr>, W, O, L, Of>;

    fn with(self, expr: Aliased<'a, Expr>) -> Self::Output {
        let source = WithQuerySource::new(self.from, expr);
        SelectStatement::new(self.select, source, self.where_clause,
            self.order, self.limit, self.offset)
    }
}
