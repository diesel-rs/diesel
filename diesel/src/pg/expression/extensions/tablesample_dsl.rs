use crate::pg::query_builder::tablesample::{BernoulliMethod, SystemMethod};
use crate::query_builder::Tablesample;
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
/// `FROM mytable TABLESAMPLE ...` --
/// `mytable.tablesample(...)` can be used just like any table in diesel since it implements
/// [Table](crate::Table).
///
/// The `BernoulliMethod` and `SystemMethod` types can be used to indicate the sampling method for
/// a `TABLESAMPLE method(p)` clause where p is specified by the portion argument. The provided
/// percentage should be an integer between 0 and 100.
///
/// To generate a `TABLESAMPLE ... REPEATABLE (f)` clause, you'll need to call
/// [`.with_seed(f)`](Tablesample::with_seed).
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
///     .tablesample_bernoulli(10)
///     .select((users::id))
///     .load::<i32>(connection);
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
///     .tablesample_system(10).with_seed(42.0)
///     .inner_join(posts::table)
///     .select((users::name, posts::title))
///     .load::<(String, String)>(connection);
/// # }
/// ```
/// That query selects all of the posts for all of the users in a random 10 percent storage pages,
/// returning the same results each time it is run due to the static seed of 42.0.
///
pub trait TablesampleDsl: Table {
    /// See the trait-level docs.
    fn tablesample_bernoulli(self, portion: i16) -> Tablesample<Self, BernoulliMethod> {
        Tablesample::new(self, portion)
    }

    /// See the trait-level docs.
    fn tablesample_system(self, portion: i16) -> Tablesample<Self, SystemMethod> {
        Tablesample::new(self, portion)
    }
}

impl<T: Table> TablesampleDsl for T {}
