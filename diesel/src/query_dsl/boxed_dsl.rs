use backend::Backend;
use query_builder::AsQuery;
use query_source::QuerySource;

pub trait InternalBoxedDsl<'a, DB: Backend> {
    type Output;

    fn internal_into_boxed(self) -> Self::Output;
}

impl<'a, T, DB> InternalBoxedDsl<'a, DB> for T where
    DB: Backend,
    T: QuerySource + AsQuery,
    T::Query: InternalBoxedDsl<'a, DB>,
{
    type Output = <T::Query as InternalBoxedDsl<'a, DB>>::Output;

    fn internal_into_boxed(self) -> Self::Output {
        self.as_query().internal_into_boxed()
    }
}

pub trait BoxedDsl: Sized {
    fn into_boxed<'a, DB>(self) -> Self::Output where
        DB: Backend,
        Self: InternalBoxedDsl<'a, DB>,
    {
        self.internal_into_boxed()
    }
}

impl<T: AsQuery> BoxedDsl for T {}
