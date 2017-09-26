use query_source::Table;

/// Sets the limit clause of a query. If there was already a limit clause, it
/// will be overridden. This is automatically implemented for the various query
/// builder types.
pub trait LimitDsl {
    type Output;

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
