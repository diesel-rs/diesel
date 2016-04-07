use query_builder::{CombinableQuery, UnionQuery};

pub trait UnionDsl<U: CombinableQuery<SqlType=Self::SqlType>>: CombinableQuery {
    type Output: CombinableQuery<SqlType=Self::SqlType>;

    fn union(self, query: U) -> Self::Output;
    fn union_all(self, query: U) -> Self::Output;
}

impl<T, U> UnionDsl<U> for T where
    T: CombinableQuery,
    U: CombinableQuery<SqlType=T::SqlType>,
{
    type Output = UnionQuery<T, U>;

    fn union(self, other: U) -> Self::Output {
        UnionQuery::new(self, other, false)
    }

    fn union_all(self, other: U) -> Self::Output {
        UnionQuery::new(self, other, true)
    }
}
