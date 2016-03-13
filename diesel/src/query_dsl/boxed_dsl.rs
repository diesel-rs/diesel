use backend::Backend;
use query_builder::AsQuery;
use query_source::QuerySource;

pub trait BoxedDsl<DB: Backend> {
    type Output;

    fn into_boxed(self) -> Self::Output;
}

impl<T, DB> BoxedDsl<DB> for T where
    DB: Backend,
    T: QuerySource + AsQuery,
    T::Query: BoxedDsl<DB>,
{
    type Output = <T::Query as BoxedDsl<DB>>::Output;

    fn into_boxed(self) -> Self::Output {
        self.as_query().into_boxed()
    }
}
