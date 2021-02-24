use crate::expression::Expression;
use crate::query_source::Table;

/// The `select` method
///
/// This trait should not be relied on directly by most apps. Its behavior is
/// provided by [`QueryDsl`]. However, you may need a where clause on this trait
/// to call `select` from generic code.
///
/// [`QueryDsl`]: crate::QueryDsl
pub trait SelectDsl<Selection: Expression> {
    // FIXME: Once we've refactored the `impl Expression` on `SelectStatement`
    // to not conditionally be `sql_types::Array`, it is probably worthwhile to
    // add a `: Expression<SqlType = Selection::SqlType>` bound here.
    /// The type returned by `.select`
    type Output;

    /// See the trait documentation
    fn select(self, selection: Selection) -> Self::Output;
}

impl<T, Selection> SelectDsl<Selection> for T
where
    Selection: Expression,
    T: Table,
    T::Query: SelectDsl<Selection>,
{
    type Output = <T::Query as SelectDsl<Selection>>::Output;

    fn select(self, selection: Selection) -> Self::Output {
        self.as_query().select(selection)
    }
}
