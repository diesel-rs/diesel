use query_builder::Aliased;
use query_source::Table;

/// The `aliased` method
///
/// This trait should not be relied on directly by most apps. Its behavior is
/// provided by [`QueryDsl`]. However, you may need a where clause on this trait
/// to call `aliased` from generic code.
///
/// [`QueryDsl`]: ../trait.QueryDsl.html
pub trait AliasedDsl<T> {
    /// The type returned by `.aliased`.
    type Output;

    /// See the trait documentation.
    fn aliased(self, alias: T) -> Self::Output;
}

impl<Tab, T> AliasedDsl<T> for Tab
where
    Tab: Table,
{
    type Output = Aliased<Self, T>;

    fn aliased(self, alias: T) -> Self::Output {
        Aliased::new(self, alias)
    }
}
