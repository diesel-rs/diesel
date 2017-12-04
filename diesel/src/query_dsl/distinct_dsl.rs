#![deny(missing_docs)]
use query_source::Table;
#[cfg(feature = "postgres")]
use expression::SelectableExpression;

/// The `distinct` method
///
/// This trait should not be relied on directly by most apps. Its behavior is
/// provided by [`QueryDsl`]. However, you may need a where clause on this trait
/// to call `distinct` from generic code.
///
/// [`QueryDsl`]: ../trait.QueryDsl.html
pub trait DistinctDsl {
    /// Query with DISTINCT added
    type Output;

    /// Adds `DISTINCT` to to the query.
    fn distinct(self) -> Self::Output;
}

impl<T> DistinctDsl for T
where
    T: Table,
    T::Query: DistinctDsl,
{
    type Output = <T::Query as DistinctDsl>::Output;

    /// Returns query with DISTINCT added
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
    /// Query with `DISTINCT ON()` added
    type Output;

    /// Should return query with `DISTINCT ON` added
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
