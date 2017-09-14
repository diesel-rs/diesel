use query_builder::AsQuery;
use query_source::Table;

/// Adds `FOR UPDATE` to the end of the select statement. This method is only
/// available for MySQL and PostgreSQL. SQLite does not provide any form of
/// row locking.
///
/// Additionally, `.for_update` cannot be used on queries with a distinct
/// clause, group by clause, having clause, or any unions. Queries with
/// a `FOR UPDATE` clause cannot be boxed.
///
/// # Example
///
/// ```ignore
/// // Executes `SELECT * FROM users FOR UPDATE`
/// users.for_update().load(&connection)
/// ```
pub trait ForUpdateDsl {
    /// The query returned by `for_update`. See [`dsl::ForUpdate`] for
    /// convenient access to this type.
    ///
    /// [`dsl::ForUpdate`]: ../dsl/type.ForUpdate.html
    type Output;

    /// See the trait level documentation
    fn for_update(self) -> Self::Output;
}

impl<T> ForUpdateDsl for T
where
    T: Table + AsQuery,
    T::Query: ForUpdateDsl,
{
    type Output = <T::Query as ForUpdateDsl>::Output;

    fn for_update(self) -> Self::Output {
        self.as_query().for_update()
    }
}
