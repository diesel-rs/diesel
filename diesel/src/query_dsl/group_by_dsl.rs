use expression::Expression;
use query_builder::{Query, AsQuery};
use query_source::QuerySource;

pub trait GroupByDsl<Expr: Expression> {
    type Output: Query;

    fn group_by(self, expr: Expr) -> Self::Output;
}

impl<T, Expr> GroupByDsl<Expr> for T where
    Expr: Expression,
    T: QuerySource + AsQuery,
    T::Query: GroupByDsl<Expr>,
{
    type Output = <T::Query as GroupByDsl<Expr>>::Output;

    fn group_by(self, expr: Expr) -> Self::Output {
        self.as_query().group_by(expr)
    }
}
