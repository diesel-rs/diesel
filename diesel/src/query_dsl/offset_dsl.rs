use query_source::Table;

/// Sets the offset clause of a query. If there was already a offset clause, it
/// will be overridden. This is automatically implemented for the various query
/// builder types.
pub trait OffsetDsl {
    type Output;

    fn offset(self, offset: i64) -> Self::Output;
}

impl<T> OffsetDsl for T
where
    T: Table,
    T::Query: OffsetDsl,
{
    type Output = <T::Query as OffsetDsl>::Output;

    fn offset(self, offset: i64) -> Self::Output {
        self.as_query().offset(offset)
    }
}
