use crate::dsl;
use crate::expression::grouped::Grouped;
use crate::expression::IntoSql;
use crate::sql_types::VarChar;
use super::operators::{Escape, Like, NotLike, SimilarTo, NotSimilarTo};

/// Adds the `escape` method to `LIKE`, `NOT LIKE`, `SIMILAR TO` and
/// `NOT SIMILAR TO`. This is used to specify the escape character for the
/// pattern.
///
/// # Example
///
/// ```rust
/// # include!("../doctest_setup.rs");
/// #
/// # fn main() {
/// #     use schema::users::dsl::*;
/// #     use diesel::insert_into;
/// #     let connection = establish_connection();
/// #     insert_into(users).values(name.eq("Ha%%0r"))
/// #         .execute(&connection).unwrap();
/// let users_with_percent = users.select(name)
///     .filter(name.ilike("%😀%%").escape('😀'))
///     .load(&connection);
/// let users_without_percent = users.select(name)
///     .filter(name.not_ilike("%a%%").escape('a'))
///     .load(&connection);
/// assert_eq!(Ok(vec![String::from("Ha%%0r")]), users_with_percent);
/// assert_eq!(Ok(vec![String::from("Sean"), String::from("Tess")]), users_without_percent);
/// # }
/// ```
pub trait PgEscapeExpressionMethods: Sized {
    #[doc(hidden)]
    type TextExpression;

    /// See the trait documentation.
    fn escape(self, _character: char) -> dsl::Escape<Self>;
}

impl<T, U> PgEscapeExpressionMethods for Grouped<ILike<T, U>> {
    type TextExpression = ILike<T, U>;

    fn escape(self, character: char) -> dsl::Escape<Self> {
        Grouped(Escape::new(
            self.0,
            character.to_string().into_sql::<VarChar>(),
        ))
    }
}

impl<T, U> PgEscapeExpressionMethods for Grouped<NotILike<T, U>> {
    type TextExpression = NotILike<T, U>;

    fn escape(self, character: char) -> dsl::Escape<Self> {
        Grouped(Escape::new(
            self.0,
            character.to_string().into_sql::<VarChar>(),
        ))
    }
}

impl<T, U> PgEscapeExpressionMethods for Grouped<SimilarTo<T, U>> {
    type TextExpression = SimilarTo<T, U>;

    fn escape(self, character: char) -> dsl::Escape<Self> {
        Grouped(Escape::new(
            self.0,
            character.to_string().into_sql::<VarChar>(),
        ))
    }
}

impl<T, U> PgEscapeExpressionMethods for Grouped<NotSimilarTo<T, U>> {
    type TextExpression = NotSimilarTo<T, U>;

    fn escape(self, character: char) -> dsl::Escape<Self> {
        Grouped(Escape::new(
            self.0,
            character.to_string().into_sql::<VarChar>(),
        ))
    }
}
