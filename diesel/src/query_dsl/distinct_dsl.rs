use query_source::Table;
use expression::SelectableExpression;

/// Adds the `DISTINCT` keyword to a query.
///
/// # Example
///
/// ```rust
///
/// # #[macro_use] extern crate diesel;
/// # include!("../doctest_setup.rs");
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

impl<T> DistinctDsl for T
where
    T: Table,
    T::Query: DistinctDsl,
{
    type Output = <T::Query as DistinctDsl>::Output;

    fn distinct(self) -> Self::Output {
        self.as_query().distinct()
    }
}

/// Adds the `DISTINCT ON` clause to a query.
///
/// # Example
///
/// ```rust
///
/// # #[macro_use] extern crate diesel;
/// # include!("../doctest_setup.rs");
/// #
/// # table! {
/// #     users {
/// #         id -> Integer,
/// #         name -> VarChar,
/// #     }
/// # }
/// #
/// # fn main() {
/// #     use self::animals::dsl::*;
/// #     let connection = establish_connection();
/// #     connection.execute("DELETE FROM animals").unwrap();
/// connection.execute("INSERT INTO animals (species, name, legs) VALUES ('dog', 'Jack', 4), ('dog', Null, 4), ('spider', Null, 8)")
///     .unwrap();
/// let all_animals = animals.select((species, name, legs)).load(&connection);
/// let distinct_animals = animals.select((species, name, legs)).distinct_on(species).load(&connection);
///
/// assert_eq!(Ok(vec![(String::from("dog"), Some(String::from("Jack")), 4),
///                    (String::from("dog"), None, 4),
///                    (String::from("spider"), None, 8)]), all_animals);
/// assert_eq!(Ok(vec![(String::from("dog"), Some(String::from("Jack")), 4),
///                    (String::from("spider"), None, 8)]), distinct_animals);
/// # }
/// ```
#[cfg(feature = "postgres")]
pub trait DistinctOnDsl<Selection: SelectableExpression<T>, T> {
    type Output;
    fn distinct_on(self, selection: Selection) -> Self::Output;
}

#[cfg(feature = "postgres")]
impl<T, Selection> DistinctOnDsl<Selection, T> for T
where
    Selection: SelectableExpression<T>,
    T: Table,
    T::Query: DistinctOnDsl<Selection, T>,
{
    type Output = <T::Query as DistinctOnDsl<Selection, T>>::Output;

    fn distinct_on(self, selection: Selection) -> Self::Output {
        self.as_query().distinct_on(selection)
    }
}
