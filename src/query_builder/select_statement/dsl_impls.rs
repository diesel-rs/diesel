use expression::*;
use query_builder::*;
use query_builder::limit_clause::*;
use query_builder::order_clause::*;
use query_builder::where_clause::*;
use query_dsl::*;
use types::{self, Bool, NativeSqlType};

impl<ST, S, F, W, O, L, Selection, Type> SelectDsl<Selection, Type>
    for SelectStatement<ST, S, F, W, O, L> where
    Selection: Expression,
    SelectStatement<Type, Selection, F, W, O, L>: Query<SqlType=Type>,
    Type: NativeSqlType,
{
    type Output = SelectStatement<Type, Selection, F, W, O, L>;

    fn select(self, selection: Selection) -> Self::Output {
        SelectStatement::new(selection, self.from, self.where_clause, self.order,
            self.limit)
    }
}

impl<ST, S, F, W, O, L, Predicate> FilterDsl<Predicate>
    for SelectStatement<ST, S, F, W, O, L> where
    Predicate: SelectableExpression<F, SqlType=Bool> + NonAggregate,
    W: WhereAnd<Predicate>,
    SelectStatement<ST, S, F, W::Output, O, L>: Query,
{
    type Output = SelectStatement<ST, S, F, W::Output, O, L>;

    fn filter(self, predicate: Predicate) -> Self::Output {
        SelectStatement::new(self.select, self.from, self.where_clause.and(predicate),
            self.order, self.limit)
    }
}

impl<ST, S, F, W, O, L, Expr> OrderDsl<Expr>
    for SelectStatement<ST, S, F, W, O, L> where
    ST: NativeSqlType,
    Expr: SelectableExpression<F>,
    SelectStatement<ST, S, F, W, OrderClause<Expr>, L>: Query<SqlType=ST>,
{
    type Output = SelectStatement<ST, S, F, W, OrderClause<Expr>, L>;

    fn order(self, expr: Expr) -> Self::Output {
        let order = OrderClause(expr);
        SelectStatement::new(self.select, self.from, self.where_clause, order,
            self.limit)
    }
}

type Limit = <i64 as AsExpression<types::BigInt>>::Expression;

impl<ST, S, F, W, O, L> LimitDsl for SelectStatement<ST, S, F, W, O, L> where
    ST: NativeSqlType,
    SelectStatement<ST, S, F, W, O, LimitClause<Limit>>: Query<SqlType=ST>,
{
    type Output = SelectStatement<ST, S, F, W, O, LimitClause<Limit>>;

    fn limit(self, limit: i64) -> Self::Output {
        let limit_clause = LimitClause(AsExpression::<types::BigInt>::as_expression(limit));
        SelectStatement::new(self.select, self.from, self.where_clause,
            self.order, limit_clause)
    }
}
