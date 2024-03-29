use crate::query_source::Table;

/// The `limit` method
///
/// This trait should not be relied on directly by most apps. Its behavior is
/// provided by [`QueryDsl`]. However, you may need a where clause on this trait
/// to call `limit` from generic code.
///
/// [`QueryDsl`]: crate::QueryDsl
pub trait LimitDsl<DummyArgForAutoType = i64> {
    /// The type returned by `.limit`
    type Output;

    /// See the trait documentation
    fn limit(self, limit: i64) -> Self::Output;
}

impl<T> LimitDsl for T
where
    T: Table,
    T::Query: LimitDsl,
{
    type Output = <T::Query as LimitDsl>::Output;

    fn limit(self, limit: i64) -> Self::Output {
        self.as_query().limit(limit)
    }
}
