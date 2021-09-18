use crate::dsl;
use crate::expression::grouped::Grouped;
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
/// # include!("../doctest_setup.rs");
/// #
/// # fn main() {
/// #     use schema::users::dsl::*;
/// #     use diesel::insert_into;
/// #     let connection = &mut establish_connection();
/// #     insert_into(users).values(name.eq("Ha%%0r"))
/// #         .execute(connection).unwrap();
/// let users_with_percent = users.select(name)
///     .filter(name.like("%😀%%").escape('😀'))
///     .load(connection);
/// let users_without_percent = users.select(name)
///     .filter(name.not_like("%a%%").escape('a'))
///     .load(connection);
/// assert_eq!(Ok(vec![String::from("Ha%%0r")]), users_with_percent);
/// assert_eq!(Ok(vec![String::from("Sean"), String::from("Tess")]), users_without_percent);
/// # }
/// ```
pub trait EscapeExpressionMethods: Sized {
    #[doc(hidden)]
    type TextExpression;

    /// See the trait documentation.
    fn escape(self, _character: char) -> dsl::Escape<Self>;
}

impl<T, U> EscapeExpressionMethods for Grouped<Like<T, U>> {
    type TextExpression = Like<T, U>;

    fn escape(self, character: char) -> dsl::Escape<Self> {
        Grouped(Escape::new(
            self.0,
            character.to_string().into_sql::<VarChar>(),
        ))
    }
}

impl<T, U> EscapeExpressionMethods for Grouped<NotLike<T, U>> {
    type TextExpression = NotLike<T, U>;

    fn escape(self, character: char) -> dsl::Escape<Self> {
        Grouped(Escape::new(
            self.0,
            character.to_string().into_sql::<VarChar>(),
        ))
    }
}
