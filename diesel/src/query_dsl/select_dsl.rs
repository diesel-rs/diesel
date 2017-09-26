use expression::Expression;
use query_source::Table;

/// Sets the select clause of a query. If there was already a select clause, it
/// will be overridden. The expression passed to `select` must actually be valid
/// for the query (only contains columns from the target table, doesn't mix
/// aggregate + non-aggregate expressions, etc).
pub trait SelectDsl<Selection: Expression> {
    // FIXME: Once we've refactored the `impl Expression` on `SelectStatement`
    // to not conditionally be `types::Array`, it is probably worthwhile to
    // add a `: Expression<SqlType = Selection::SqlType>` bound here.
    type Output;

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
