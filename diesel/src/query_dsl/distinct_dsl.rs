use query_builder::AsQuery;
use query_source::QuerySource;

/// Adds the `DISTINCT` keyword to a query.
///
/// # Example
///
/// ```rust
///
/// # #[macro_use] extern crate diesel;
/// # include!("src/doctest_setup.rs");
/// #
/// # table! {
/// #     users {
/// #         id -> Integer,
/// #         name -> VarChar,
/// #     }
/// # }
/// #
/// # fn main() {
/// #     use self::users::dsl::*;
/// #     let connection = establish_connection();
/// #     connection.execute("DELETE FROM users").unwrap();
/// connection.execute("INSERT INTO users (name) VALUES ('Sean'), ('Sean'), ('Sean')")
///     .unwrap();
/// let names = users.select(name).load(&connection);
/// let distinct_names = users.select(name).distinct().load(&connection);
///
/// let sean = String::from("Sean");
/// assert_eq!(Ok(vec![sean.clone(), sean.clone(), sean.clone()]), names);
/// assert_eq!(Ok(vec![sean.clone()]), distinct_names);
/// # }
/// ```
pub trait DistinctDsl {
    type Output;
    fn distinct(self) -> Self::Output;
}

impl<T> DistinctDsl for T where
    T: AsQuery + QuerySource,
    T::Query: DistinctDsl,
{
    type Output = <T::Query as DistinctDsl>::Output;

    fn distinct(self) -> Self::Output {
        self.as_query().distinct()
    }
}
