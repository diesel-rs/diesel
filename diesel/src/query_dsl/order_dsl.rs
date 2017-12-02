use expression::Expression;
use query_source::Table;

/// The `order` method
///
/// This trait should not be relied on directly by most apps. Its behavior is
/// provided by [`QueryDsl`]. However, you may need a where clause on this trait
/// to call `order` from generic code.
///
/// [`QueryDsl`]: ../trait.QueryDsl.html
pub trait OrderDsl<Expr: Expression> {
    type Output;

    fn order(self, expr: Expr) -> Self::Output;
}

impl<T, Expr> OrderDsl<Expr> for T
where
    Expr: Expression,
    T: Table,
    T::Query: OrderDsl<Expr>,
{
    type Output = <T::Query as OrderDsl<Expr>>::Output;

    fn order(self, expr: Expr) -> Self::Output {
        self.as_query().order(expr)
    }
}
