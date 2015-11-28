use query_builder::{Query, AsQuery};
use query_source::QuerySource;

pub trait OffsetDsl {
    type Output: Query;

    fn offset(self, offset: i64) -> Self::Output;
}

impl<T> OffsetDsl for T where
    T: QuerySource + AsQuery,
    T::Query: OffsetDsl,
{
    type Output = <T::Query as OffsetDsl>::Output;

    fn offset(self, offset: i64) -> Self::Output {
        self.as_query().offset(offset)
    }
}
