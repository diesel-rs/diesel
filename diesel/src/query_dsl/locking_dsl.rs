use query_builder::AsQuery;
use query_source::Table;

/// The `for_update` method
///
/// This trait should not be relied on directly by most apps. Its behavior is
/// provided by [`QueryDsl`]. However, you may need a where clause on this trait
/// to call `for_update` from generic code.
///
/// [`QueryDsl`]: ../trait.QueryDsl.html
pub trait ForUpdateDsl {
    /// The query returned by `for_update`. See [`dsl::ForUpdate`] for
    /// convenient access to this type.
    ///
    /// [`dsl::ForUpdate`]: ../dsl/type.ForUpdate.html
    type Output;

    /// See the trait level documentation
    fn for_update(self) -> Self::Output;
}

impl<T> ForUpdateDsl for T
where
    T: Table + AsQuery,
    T::Query: ForUpdateDsl,
{
    type Output = <T::Query as ForUpdateDsl>::Output;

    fn for_update(self) -> Self::Output {
        self.as_query().for_update()
    }
}
