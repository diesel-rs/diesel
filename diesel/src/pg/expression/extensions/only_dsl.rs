use crate::query_builder::{Only, SelectStatement};
use crate::query_source::{QuerySource, Table};

/// The `only` method
///
/// This is only implemented for the Postgres backend.
/// The `ONLY` clause is used to select only from one table and not any inherited ones.
///
/// Calling this function on a table (`mytable.only()`) will result in the SQL `FROM ONLY mytable`.
///
/// Example:
///
/// ```ignore
/// let n_users_in_main_table = users::table
///     .only()
///     .select(count(users::id))
///     .first::<i64>(connection)
///     .unwrap();
///
/// ```
/// Selects the number of entries in the `users` table excluding any rows found in inherited
/// tables.
pub trait OnlyDsl: Table + Sized {
    /// See the trait-level docs.
    fn only(self) -> crate::dsl::SelectFromOnly<Self>
    where
        Only<Self>: QuerySource,
    {
        SelectStatement::simple(Only { query: self })
    }
}

impl<T: Table> OnlyDsl for T {}
