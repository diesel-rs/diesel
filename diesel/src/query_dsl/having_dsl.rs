use crate::dsl;

/// The `having` method
///
/// This trait should not be relied on directly by most apps. Its behavior is
/// provided by [`QueryDsl`]. However, you may need a where clause on this trait
/// to call `having` from generic code.
///
/// [`QueryDsl`]: crate::QueryDsl
pub trait HavingDsl<Predicate> {
    /// The type returned by `.having`.
    type Output;

    /// See the trait documentation.
    fn having(self, predicate: Predicate) -> dsl::Having<Self, Predicate>;
}
