use expression::{Expression, AsExpression};
use super::predicates::*;

pub trait PgExpressionMethods: Expression + Sized {
    /// Creates a PostgreSQL `IS NOT DISTINCT FROM` expression. This behaves
    /// identically to the `=` operator, except that `NULL` is treated as a
    /// normal value.
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
    /// let data = users.select(id).filter(name.is_not_distinct_from("Sean"));
    /// assert_eq!(Ok(1), data.first(&connection));
    /// # }
    fn is_not_distinct_from<T>(self, other: T)
        -> IsNotDistinctFrom<Self, T::Expression> where
            T: AsExpression<Self::SqlType>,
    {
        IsNotDistinctFrom::new(self, other.as_expression())
    }
}

impl<T: Expression> PgExpressionMethods for T {}

use super::date_and_time::AtTimeZone;
use types::{VarChar, Timestamp};

#[doc(hidden)]
pub trait PgTimestampExpressionMethods: Expression<SqlType=Timestamp> + Sized {
    /// Returns a PostgreSQL "AT TIME ZONE" expression
    fn at_time_zone<T>(self, timezone: T) -> AtTimeZone<Self, T::Expression> where
        T: AsExpression<VarChar>,
    {
        AtTimeZone::new(self, timezone.as_expression())
    }
}

impl<T: Expression<SqlType=Timestamp>> PgTimestampExpressionMethods for T {}
