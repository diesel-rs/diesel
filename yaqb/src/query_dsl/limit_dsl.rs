use query_builder::{Query, AsQuery};
use query_source::QuerySource;

pub trait LimitDsl {
    type Output: Query;

    fn limit(self, limit: i64) -> Self::Output;
}

impl<T> LimitDsl for T where
    T: QuerySource + AsQuery,
    T::Query: LimitDsl,
{
    type Output = <T::Query as LimitDsl>::Output;

    fn limit(self, limit: i64) -> Self::Output {
        self.as_query().limit(limit)
    }
}
