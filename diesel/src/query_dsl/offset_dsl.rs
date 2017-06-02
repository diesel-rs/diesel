use query_builder::AsQuery;
use query_source::Table;

/// Sets the offset clause of a query. If there was already a offset clause, it
/// will be overridden. This is automatically implemented for the various query
/// builder types.
pub trait OffsetDsl: AsQuery {
    type Output: AsQuery<SqlType=Self::SqlType>;

    fn offset(self, offset: i64) -> Self::Output;
}

impl<T, ST> OffsetDsl for T where
    T: Table + AsQuery<SqlType=ST>,
    T::Query: OffsetDsl<SqlType=ST>,
{
    type Output = <T::Query as OffsetDsl>::Output;

    fn offset(self, offset: i64) -> Self::Output {
        self.as_query().offset(offset)
    }
}
