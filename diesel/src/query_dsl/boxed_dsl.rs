use crate::query_builder::AsQuery;
use crate::query_source::Table;

/// The `into_boxed` method
///
/// This trait should not be relied on directly by most apps. Its behavior is
/// provided by [`QueryDsl`]. However, you may need a where clause on this trait
/// to call `into_boxed` from generic code.
///
/// [`QueryDsl`]: ../trait.QueryDsl.html
pub trait BoxedDsl<'a, DB> {
    /// The return type of `internal_into_boxed`
    type Output;

    /// See the trait documentation.
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
