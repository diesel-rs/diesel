use crate::expression::Expression;
use crate::query_source::Table;

/// The `order` method
///
/// This trait should not be relied on directly by most apps. Its behavior is
/// provided by [`QueryDsl`]. However, you may need a where clause on this trait
/// to call `order` from generic code.
///
/// [`QueryDsl`]: crate::QueryDsl
pub trait OrderDsl<Expr: Expression> {
    /// The type returned by `.order`.
    type Output;

    /// See the trait documentation.
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

/// The `then_order_by` method
///
/// This trait should not be relied on directly by most apps. Its behavior is
/// provided by [`QueryDsl`]. However, you may need a where clause on this trait
/// to call `then_order_by` from generic code.
///
/// [`QueryDsl`]: crate::QueryDsl
pub trait ThenOrderDsl<Expr> {
    /// The type returned by `.then_order_by`.
    type Output;

    /// See the trait documentation.
    fn then_order_by(self, expr: Expr) -> Self::Output;
}

impl<T, Expr> ThenOrderDsl<Expr> for T
where
    Expr: Expression,
    T: Table,
    T::Query: ThenOrderDsl<Expr>,
{
    type Output = <T::Query as ThenOrderDsl<Expr>>::Output;

    fn then_order_by(self, expr: Expr) -> Self::Output {
        self.as_query().then_order_by(expr)
    }
}

pub trait ValidOrderingForDistinct<D> {}
