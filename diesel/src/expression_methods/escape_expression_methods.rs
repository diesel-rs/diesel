use crate::dsl::AsExprOf;
use crate::expression::operators::{Escape, Like, NotLike};
use crate::expression::IntoSql;
use crate::sql_types::VarChar;

/// Adds the `escape` method to `LIKE` and `NOT LIKE`. This is used to specify
/// the escape character for the pattern.
///
/// By default, the escape character is `\` on most backends. On SQLite,
/// there is no default escape character.
///
/// # Example
///
/// ```rust
/// # #[macro_use] extern crate diesel;
/// # include!("../doctest_setup.rs");
/// #
/// # fn main() {
/// #     use schema::users::dsl::*;
/// #     use diesel::insert_into;
/// #     let connection = establish_connection();
/// #     insert_into(users).values(name.eq("Ha%%0r"))
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
    /// See the trait documentation.
    fn escape(self, character: char) -> Escape<Self, AsExprOf<String, VarChar>> {
        Escape::new(self, character.to_string().into_sql::<VarChar>())
    }
}

impl<T, U> EscapeExpressionMethods for Like<T, U> {}

impl<T, U> EscapeExpressionMethods for NotLike<T, U> {}
