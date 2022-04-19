use crate::query_builder::Only;
use crate::Table;

/// The `only` method
///
/// This is only implemented for the Postgres backend.
/// The `ONLY` clause is used to select only from one table and not any inherited ones.
///
/// Calling this function on a table (`mytable.only()`) will result in the SQL `ONLY mytable`.
/// `mytable.only()` can be used just like any table in diesel since it implements
/// [Table](crate::Table).
///
/// Example:
///
/// ```rust
/// # include!("../../../doctest_setup.rs");
/// # use schema::{posts, users};
/// # use diesel::dsl::*;
/// # fn main() {
/// # let connection = &mut establish_connection();
/// let n_sers_in_main_table = users::table
///     .only()
///     .select(count(users::id))
///     .first::<i64>(connection);
/// # }
/// ```
/// Selects the number of entries in the `users` table excluding any rows found in inherited
/// tables.
///
/// It can also be used in inner joins:
///
/// ```rust
/// # include!("../../../doctest_setup.rs");
/// # use schema::{posts, users};
/// # use diesel::dsl::*;
/// # fn main() {
/// # let connection = &mut establish_connection();
/// # let _ =
/// users::table
///     .inner_join(posts::table.only())
///     .select((users::name, posts::title))
///     .load::<(String, String)>(connection);
/// # }
/// ```
/// That query excludes any posts that reside in any inherited table.
///
pub trait OnlyDsl: Table {
    /// See the trait-level docs.
    fn only(self) -> Only<Self> {
        Only { source: self }
    }
}

impl<T: Table> OnlyDsl for T {}
