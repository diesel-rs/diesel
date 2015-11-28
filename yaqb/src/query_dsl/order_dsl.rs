use expression::Expression;
use query_builder::{Query, AsQuery};
use query_source::QuerySource;

pub trait OrderDsl<Expr: Expression> {
    type Output: Query;

    fn order(self, expr: Expr) -> Self::Output;
}

impl<T, Expr> OrderDsl<Expr> for T where
    Expr: Expression,
    T: QuerySource + AsQuery,
    T::Query: OrderDsl<Expr>,
{
    type Output = <T::Query as OrderDsl<Expr>>::Output;

    fn order(self, expr: Expr) -> Self::Output {
        self.as_query().order(expr)
    }
}
