use crate::query_builder::Tablesample;
pub(crate) use crate::query_builder::{TablesampleMethod, TablesampleSeed};
use crate::Table;

/// The `tablesample` method
///
/// The `TABLESAMPLE` clause is used to select a randomly sampled subset of rows from a table.
///
/// This is only implemented for the Postgres backend. While `TABLESAMPLE` is standardized in
/// SQL:2003, in practice each RDBMS seems to implement a superset of the SQL:2003 syntax,
/// supporting a wide variety of sampling methods.
///
/// Calling this function on a table (`mytable.tablesample(...)`) will result in the SQL
/// `FROM mytable TABLESAMPLE ...`.
/// `mytable.tablesample(...)` can be used just like any table in diesel since it implements
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
/// let random_user_ids = users::table
///     .tablesample(TablesampleMethod::Bernoulli(10), TablesampleSeed::Auto)
///     .select(users::id)
///     .load::<i64>(connection);
/// # }
/// ```
/// Selects the ids for a random 10 percent of users.
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
///     .tablesample(TablesampleMethod::Bernoulli(10), TablesampleSeed::Auto)
///     .inner_join(posts::table.only())
///     .select((users::name, posts::title))
///     .load::<(String, String)>(connection);
/// # }
/// ```
/// That query selects all of the posts for a random 10 percent of users.
///
pub trait TablesampleDsl: Table {
    /// See the trait-level docs.
    fn tablesample(self, method: TablesampleMethod, seed: TablesampleSeed) -> Tablesample<Self> {
        Tablesample {
            source: self,
            method,
            seed,
        }
    }
}

impl<T: Table> TablesampleDsl for T {}
