use crate::expression::Expression;
use crate::query_builder::AsQuery;
use crate::query_source::Table;

/// The `group_by` method
///
/// This trait should not be relied on directly by most apps. Its behavior is
/// provided by [`QueryDsl`]. However, you may need a where clause on this trait
/// to call `group_by` from generic code.
///
/// [`QueryDsl`]: ../trait.QueryDsl.html
pub trait GroupByDsl<Expr: Expression> {
    /// The type returned by `.group_by`
    type Output;

    /// See the trait documentation.
    fn group_by(self, expr: Expr) -> Self::Output;
}

impl<T, Expr> GroupByDsl<Expr> for T
where
    Expr: Expression,
    T: Table + AsQuery,
    T::Query: GroupByDsl<Expr>,
{
    type Output = <T::Query as GroupByDsl<Expr>>::Output;

    fn group_by(self, expr: Expr) -> Self::Output {
        self.as_query().group_by(expr)
    }
}
