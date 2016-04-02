use backend::Backend;
use query_builder::AsQuery;
use query_source::QuerySource;

pub trait InternalBoxedDsl<DB: Backend> {
    type Output;

    fn internal_into_boxed(self) -> Self::Output;
}

impl<T, DB> InternalBoxedDsl<DB> for T where
    DB: Backend,
    T: QuerySource + AsQuery,
    T::Query: InternalBoxedDsl<DB>,
{
    type Output = <T::Query as InternalBoxedDsl<DB>>::Output;

    fn internal_into_boxed(self) -> Self::Output {
        self.as_query().internal_into_boxed()
    }
}

pub trait BoxedDsl: Sized {
    fn into_boxed<DB>(self) -> Self::Output where
        DB: Backend,
        Self: InternalBoxedDsl<DB>,
    {
        self.internal_into_boxed()
    }
}

impl<T: AsQuery> BoxedDsl for T {}
