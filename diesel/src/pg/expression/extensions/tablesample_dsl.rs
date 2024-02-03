use crate::query_builder::Tablesample;
pub(crate) use crate::query_builder::TablesampleMethod;
use crate::Table;
use std::marker::PhantomData;

#[derive(Clone, Copy, Debug)]
/// Used to specify the `BERNOULLI` sampling method.
pub struct BernoulliMethod;

impl TablesampleMethod for BernoulliMethod {
    fn method_name_sql() -> &'static str {
        "BERNOULLI"
    }
}

#[derive(Clone, Copy, Debug)]
/// Used to specify the `SYSTEM` sampling method.
pub struct SystemMethod;

impl TablesampleMethod for SystemMethod {
    fn method_name_sql() -> &'static str {
        "SYSTEM"
    }
}

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
/// If the seed argument is is Some(f) then f becomes the seed in `TABLESAMPLE ... REPEATABLE (f)`.
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
///     .tablesample::<BernoulliMethod>(10, None)
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
///     .tablesample::<BernoulliMethod>(10, Some(42.0))
///     .inner_join(posts::table)
///     .select((users::name, posts::title))
///     .load::<(String, String)>(connection);
/// # }
/// ```
/// That query selects all of the posts for a random 10 percent of users, returning the same
/// results each time it is run due to the static seed of 42.0.
///
pub trait TablesampleDsl: Table {
    /// See the trait-level docs.
    fn tablesample<TSM: TablesampleMethod>(
        self,
        portion: i16,
        seed: Option<f64>,
    ) -> Tablesample<Self, TSM> {
        Tablesample {
            source: self,
            method: PhantomData,
            portion,
            seed,
        }
    }
}

impl<T: Table> TablesampleDsl for T {}
