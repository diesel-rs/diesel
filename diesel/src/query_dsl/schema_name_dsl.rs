use crate::query_source::Table;

/// The `schema_name` method
///
/// This trait should not be relied on directly by most apps. Its behavior is
/// provided by [`QueryDsl`]. However, you may need a from clause on this trait
/// to call `limit` from generic code.
///
/// [`QueryDsl`]: crate::QueryDsl
pub trait SchemaNameDsl {
    /// The type returned by schema_name method
    type Output;

    /// See the trait documentation
    fn schema_name(self, schema_name: &'_ String) -> Self::Output;
}

impl<T> SchemaNameDsl for T
where
    T: Table,
    T::Query: SchemaNameDsl,
{
    type Output = <T::Query as SchemaNameDsl>::Output;

    fn schema_name(self, schema_name: &'_ String) -> Self::Output {
        self.as_query().schema_name(schema_name)
    }
}
