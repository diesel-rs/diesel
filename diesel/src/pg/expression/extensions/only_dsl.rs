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
/// ```
/// # include!("../../../doctest_setup.rs");
/// # use crate::schema::users;
/// # use diesel::dsl::OnlyDsl;
/// # use diesel::dsl::count;
/// #
/// # fn test() -> QueryResult<()> {
/// # let connection = &mut establish_connection();
/// #
/// let n_users_in_main_table = users::table
///     .only()
///     .select(count(users::id))
///     .first::<i64>(connection)?;
/// #
/// # Ok(())
/// # }
/// # fn main() {
/// #     test().unwrap();
/// # }
/// ```
/// Selects the number of entries in the `users` table excluding any rows found in inherited
/// tables.
#[cfg(feature = "postgres_backend")]
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
