use expression::*;
use query_builder::*;
use query_builder::where_clause::*;
use query_builder::order_clause::*;
use query_dsl::*;
use types::{Bool, NativeSqlType};

impl<ST, S, F, W, O, Selection, Type> SelectDsl<Selection, Type>
    for SelectStatement<ST, S, F, W, O> where
    Selection: Expression,
    SelectStatement<Type, Selection, F, W, O>: Query<SqlType=Type>,
    Type: NativeSqlType,
{
    type Output = SelectStatement<Type, Selection, F, W, O>;

    fn select(self, selection: Selection) -> Self::Output {
        SelectStatement::new(selection, self.from, self.where_clause, self.order)
    }
}

impl<ST, S, F, W, O, Predicate> FilterDsl<Predicate>
    for SelectStatement<ST, S, F, W, O> where
    Predicate: SelectableExpression<F, SqlType=Bool> + NonAggregate,
    W: WhereAnd<Predicate>,
    SelectStatement<ST, S, F, W::Output, O>: Query,
{
    type Output = SelectStatement<ST, S, F, W::Output, O>;

    fn filter(self, predicate: Predicate) -> Self::Output {
        SelectStatement::new(self.select, self.from, self.where_clause.and(predicate),
            self.order)
    }
}

impl<ST, S, F, W, O, Expr> OrderDsl<Expr>
    for SelectStatement<ST, S, F, W, O> where
    ST: NativeSqlType,
    Expr: SelectableExpression<F>,
    SelectStatement<ST, S, F, W, OrderClause<Expr>>: Query<SqlType=ST>,
{
    type Output = SelectStatement<ST, S, F, W, OrderClause<Expr>>;

    fn order(self, expr: Expr) -> Self::Output {
        let order = OrderClause(expr);
        SelectStatement::new(self.select, self.from, self.where_clause, order)
    }
}
