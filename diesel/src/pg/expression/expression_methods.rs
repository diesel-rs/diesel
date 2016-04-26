use expression::{Expression, AsExpression};
use super::predicates::*;
use types::Array;

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

pub trait ArrayExpressionMethods<ST>: Expression<SqlType=Array<ST>> + Sized {
    /// Compares two arrays for common elements, using the `&&` operator in
    /// the final SQL
    ///
    /// # Example
    ///
    /// ```rust
    /// # #[macro_use] extern crate diesel;
    /// # include!("src/doctest_setup.rs");
    /// #
    /// # table! {
    /// #     posts {
    /// #         id -> Integer,
    /// #         tags -> Array<VarChar>,
    /// #     }
    /// # }
    /// #
    /// # // FIXME: We shouldn't need to define a users table here
    /// # table! {
    /// #     users {
    /// #         id -> Integer,
    /// #         name -> VarChar,
    /// #     }
    /// # }
    /// #
    /// # struct NewPost<'a> { tags: Vec<&'a str> }
    /// # Insertable! { (posts) struct NewPost<'a> { tags: Vec<&'a str>, } }
    /// #
    /// # fn main() {
    /// #     use self::posts::dsl::*;
    /// #     let conn = establish_connection();
    /// #     conn.execute("DROP TABLE IF EXISTS posts").unwrap();
    /// #     conn.execute("CREATE TABLE posts (id SERIAL PRIMARY KEY, tags TEXT[] NOT NULL)").unwrap();
    /// #
    /// diesel::insert(&vec![
    ///     NewPost { tags: vec!["cool", "awesome"] },
    ///     NewPost { tags: vec!["awesome", "great"] },
    ///     NewPost { tags: vec!["cool", "great"] },
    /// ]).into(posts).execute(&conn).unwrap();
    ///
    /// let query = posts.select(id).filter(tags.overlaps_with(vec!["horrid", "cool"]));
    /// assert_eq!(Ok(vec![1, 3]), query.load(&conn));
    ///
    /// let query = posts.select(id).filter(tags.overlaps_with(vec!["cool", "great"]));
    /// assert_eq!(Ok(vec![1, 2, 3]), query.load(&conn));
    ///
    /// let query = posts.select(id).filter(tags.overlaps_with(vec!["horrid"]));
    /// assert_eq!(Ok(Vec::new()), query.load::<i32>(&conn));
    /// # }
    /// ```
    fn overlaps_with<T>(self, other: T) -> OverlapsWith<Self, T::Expression> where
        T: AsExpression<Self::SqlType>,
    {
        OverlapsWith::new(self, other.as_expression())
    }

    /// Compares whether an array contains another array, using the `@>` operator.
    ///
    /// # Example
    ///
    /// ```rust
    /// # #[macro_use] extern crate diesel;
    /// # include!("src/doctest_setup.rs");
    /// #
    /// # table! {
    /// #     posts {
    /// #         id -> Integer,
    /// #         tags -> Array<VarChar>,
    /// #     }
    /// # }
    /// #
    /// # // FIXME: We shouldn't need to define a users table here
    /// # table! {
    /// #     users {
    /// #         id -> Integer,
    /// #         name -> VarChar,
    /// #     }
    /// # }
    /// #
    /// # struct NewPost<'a> { tags: Vec<&'a str> }
    /// # Insertable! { (posts) struct NewPost<'a> { tags: Vec<&'a str>, } }
    /// #
    /// # fn main() {
    /// #     use self::posts::dsl::*;
    /// #     let conn = establish_connection();
    /// #     conn.execute("DROP TABLE IF EXISTS posts").unwrap();
    /// #     conn.execute("CREATE TABLE posts (id SERIAL PRIMARY KEY, tags TEXT[] NOT NULL)").unwrap();
    /// #
    /// diesel::insert(&vec![
    ///     NewPost { tags: vec!["cool", "awesome"] },
    /// ]).into(posts).execute(&conn).unwrap();
    ///
    /// let query = posts.select(id).filter(tags.contains(vec!["cool"]));
    /// assert_eq!(Ok(vec![1]), query.load(&conn));
    ///
    /// let query = posts.select(id).filter(tags.contains(vec!["cool", "amazing"]));
    /// assert_eq!(Ok(Vec::new()), query.load::<i32>(&conn));
    /// # }
    /// ```
    fn contains<T>(self, other: T) -> Contains<Self, T::Expression> where
        T: AsExpression<Self::SqlType>,
    {
        Contains::new(self, other.as_expression())
    }

    /// Compares whether an array is contained by another array, using the `<@` operator.
    /// This is the opposite of `contains`
    ///
    /// # Example
    ///
    /// ```rust
    /// # #[macro_use] extern crate diesel;
    /// # include!("src/doctest_setup.rs");
    /// #
    /// # table! {
    /// #     posts {
    /// #         id -> Integer,
    /// #         tags -> Array<VarChar>,
    /// #     }
    /// # }
    /// #
    /// # // FIXME: We shouldn't need to define a users table here
    /// # table! {
    /// #     users {
    /// #         id -> Integer,
    /// #         name -> VarChar,
    /// #     }
    /// # }
    /// #
    /// # struct NewPost<'a> { tags: Vec<&'a str> }
    /// # Insertable! { (posts) struct NewPost<'a> { tags: Vec<&'a str>, } }
    /// #
    /// # fn main() {
    /// #     use self::posts::dsl::*;
    /// #     let conn = establish_connection();
    /// #     conn.execute("DROP TABLE IF EXISTS posts").unwrap();
    /// #     conn.execute("CREATE TABLE posts (id SERIAL PRIMARY KEY, tags TEXT[] NOT NULL)").unwrap();
    /// #
    /// diesel::insert(&vec![
    ///     NewPost { tags: vec!["cool", "awesome"] },
    /// ]).into(posts).execute(&conn).unwrap();
    ///
    /// let query = posts.select(id).filter(tags.is_contained_by(vec!["cool", "awesome", "amazing"]));
    /// assert_eq!(Ok(vec![1]), query.load(&conn));
    ///
    /// let query = posts.select(id).filter(tags.is_contained_by(vec!["cool"]));
    /// assert_eq!(Ok(Vec::new()), query.load::<i32>(&conn));
    /// # }
    /// ```
    fn is_contained_by<T>(self, other: T) -> IsContainedBy<Self, T::Expression> where
        T: AsExpression<Self::SqlType>,
    {
        IsContainedBy::new(self, other.as_expression())
    }
}

impl<T, ST> ArrayExpressionMethods<ST> for T where
    T: Expression<SqlType=Array<ST>>,
{
}
