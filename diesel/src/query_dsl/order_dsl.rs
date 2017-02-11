use expression::Expression;
use query_builder::AsQuery;
use query_source::QuerySource;

/// Sets the order clause of a query. If there was already a order clause, it
/// will be overridden. The expression passed to `order` must actually be valid
/// for the query. See also:
/// [`.desc()`](../../expression/expression_methods/global_expression_methods/trait.ExpressionMethods.html#method.desc)
/// and [`.asc()`](../../expression/expression_methods/global_expression_methods/trait.ExpressionMethods.html#method.asc)
///
/// Ordering by multiple columns can be achieved by passing a tuple of those
/// columns.
///
/// This is automatically implemented for the various query builder types.
pub trait OrderDsl<Expr: Expression>: AsQuery {
    type Output: AsQuery<SqlType=Self::SqlType>;

    fn order(self, expr: Expr) -> Self::Output;
}

impl<T, Expr, ST> OrderDsl<Expr> for T where
    Expr: Expression,
    T: QuerySource + AsQuery<SqlType=ST>,
    T::Query: OrderDsl<Expr, SqlType=ST>,
{
    type Output = <T::Query as OrderDsl<Expr>>::Output;

    fn order(self, expr: Expr) -> Self::Output {
        self.as_query().order(expr)
    }
}
