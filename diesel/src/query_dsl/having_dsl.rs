use crate::dsl::Having;
use crate::query_source::*;

/// The `having` method
///
/// This trait should not be relied on directly by most apps. Its behavior is
/// provided by [`QueryDsl`]. However, you may need a where clause on this trait
/// to call `having` from generic code.
///
/// [`QueryDsl`]: ../trait.QueryDsl.html
pub trait HavingDsl<Predicate> {
    /// The type returned by `.having`.
    type Output;

    /// See the trait documentation.
    fn having(self, predicate: Predicate) -> Self::Output;
}

impl<T, Predicate> HavingDsl<Predicate> for T
where
    T: Table,
    T::Query: HavingDsl<Predicate>,
{
    type Output = Having<T::Query, Predicate>;

    fn having(self, predicate: Predicate) -> Self::Output {
        self.as_query().having(predicate)
    }
}
