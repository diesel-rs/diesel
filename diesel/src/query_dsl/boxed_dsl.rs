use backend::Backend;
use query_builder::AsQuery;
use query_source::Table;

pub trait BoxedDsl<'a, DB: Backend> {
    type Output;

    fn internal_into_boxed(self) -> Self::Output;
}

impl<'a, T, DB> BoxedDsl<'a, DB> for T
where
    DB: Backend,
    T: Table + AsQuery,
    T::Query: BoxedDsl<'a, DB>,
{
    type Output = <T::Query as BoxedDsl<'a, DB>>::Output;

    fn internal_into_boxed(self) -> Self::Output {
        self.as_query().internal_into_boxed()
    }
}
