use expression::*;
use query_builder::{Query, AsQuery};
use query_source::Table;

/// Sets the select clause of a query. If there was already a select clause, it
/// will be overridden. The expression passed to `select` must actually be valid
/// for the query (only contains columns from the target table, doesn't mix
/// aggregate + non-aggregate expressions, etc).
pub trait SelectDsl<Selection: Expression> {
    type Output: Query<SqlType=<Selection as Expression>::SqlType>;

    fn select(self, selection: Selection) -> Self::Output;
}

impl<T, Selection> SelectDsl<Selection> for T where
    Selection: Expression,
    T: Table + AsQuery,
    T::Query: SelectDsl<Selection>,
{
    type Output = <T::Query as SelectDsl<Selection>>::Output;

    fn select(self, selection: Selection) -> Self::Output {
        self.as_query().select(selection)
    }
}
