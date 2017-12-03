use query_builder::AsQuery;
use query_source::Table;

pub trait BoxedDsl<'a, DB> {
    type Output;

    fn internal_into_boxed(self) -> Self::Output;
}

impl<'a, T, DB> BoxedDsl<'a, DB> for T
where
    T: Table + AsQuery,
    T::Query: BoxedDsl<'a, DB>,
{
    type Output = <T::Query as BoxedDsl<'a, DB>>::Output;

    fn internal_into_boxed(self) -> Self::Output {
        self.as_query().internal_into_boxed()
    }
}
