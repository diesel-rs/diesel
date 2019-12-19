#[cfg(feature = "postgres")]
use crate::expression::SelectableExpression;
use crate::query_source::Table;

/// The `distinct` method
///
/// This trait should not be relied on directly by most apps. Its behavior is
/// provided by [`QueryDsl`]. However, you may need a where clause on this trait
/// to call `distinct` from generic code.
///
/// [`QueryDsl`]: ../trait.QueryDsl.html
pub trait DistinctDsl {
    /// The type returned by `.distinct`
    type Output;

    /// See the trait documentation.
    fn distinct(self) -> Self::Output;
}

impl<T> DistinctDsl for T
where
    T: Table,
    T::Query: DistinctDsl,
{
    type Output = <T::Query as DistinctDsl>::Output;

    fn distinct(self) -> Self::Output {
        self.as_query().distinct()
    }
}

/// The `distinct_on` method
///
/// This trait should not be relied on directly by most apps. Its behavior is
/// provided by [`QueryDsl`]. However, you may need a where clause on this trait
/// to call `distinct_on` from generic code.
///
/// [`QueryDsl`]: ../trait.QueryDsl.html
#[cfg(feature = "postgres")]
pub trait DistinctOnDsl<Selection> {
    /// The type returned by `.distinct_on`
    type Output;

    /// See the trait documentation
    fn distinct_on(self, selection: Selection) -> Self::Output;
}

#[cfg(feature = "postgres")]
impl<T, Selection> DistinctOnDsl<Selection> for T
where
    Selection: SelectableExpression<T>,
    T: Table,
    T::Query: DistinctOnDsl<Selection>,
{
    type Output = <T::Query as DistinctOnDsl<Selection>>::Output;

    fn distinct_on(self, selection: Selection) -> Self::Output {
        self.as_query().distinct_on(selection)
    }
}
