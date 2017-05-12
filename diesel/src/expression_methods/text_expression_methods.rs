use expression::{Expression, AsExpression};
use expression::predicates::{Concat, Like, NotLike};
use types::Text;

pub trait TextExpressionMethods: Expression<SqlType=Text> + Sized {
    /// Concatenates two strings using the `||` operator.
    ///
    /// # Example
    ///
    /// ```rust
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
    /// #
    /// let names = users.select(name.concat(" the Greatest")).load(&connection);
    /// let expected_names = vec![
    ///     "Sean the Greatest".to_string(),
    ///     "Tess the Greatest".to_string(),
    /// ];
    /// assert_eq!(Ok(expected_names), names);
    /// # }
    /// ```
    fn concat<T: AsExpression<Text>>(self, other: T) -> Concat<Self, T::Expression> {
        Concat::new(self, other.as_expression())
    }

    /// Returns a SQL `LIKE` expression
    fn like<T: AsExpression<Text>>(self, other: T) -> Like<Self, T::Expression> {
        Like::new(self.as_expression(), other.as_expression())
    }

    /// Returns a SQL `NOT LIKE` expression
    fn not_like<T: AsExpression<Text>>(self, other: T) -> NotLike<Self, T::Expression> {
        NotLike::new(self.as_expression(), other.as_expression())
    }
}

impl<T: Expression<SqlType=Text>> TextExpressionMethods for T {}
