use dsl::AsExprOf;
use expression::IntoSql;
use expression::operators::{Escape, Like, NotLike};
use types::VarChar;
/// Adds the `escape` method to `LIKE` and `NOT LIKE`. This is used to specify
/// the escape character for the pattern.
///
/// # Example
///
/// ```rust
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
/// #     use diesel::insert_into;
/// #     let connection = establish_connection();
/// #     insert_into(users).values(&NewUser { name: "Ha%%0r".into() })
/// #         .execute(&connection).unwrap();
/// let users_with_percent = users.select(name)
///     .filter(name.like("%😀%%").escape('😀'))
///     .load(&connection);
/// let users_without_percent = users.select(name)
///     .filter(name.not_like("%a%%").escape('a'))
///     .load(&connection);
/// assert_eq!(Ok(vec![String::from("Ha%%0r")]), users_with_percent);
/// assert_eq!(Ok(vec![String::from("Sean"), String::from("Tess")]), users_without_percent);
/// # }
/// ```
pub trait EscapeExpressionMethods: Sized {
    fn escape(self, character: char) -> Escape<Self, AsExprOf<String, VarChar>> {
        Escape::new(self, character.to_string().into_sql::<VarChar>())
    }
}

impl<T, U> EscapeExpressionMethods for Like<T, U> {}

impl<T, U> EscapeExpressionMethods for NotLike<T, U> {}
