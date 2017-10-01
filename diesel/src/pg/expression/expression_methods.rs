use expression::{AsExpression, Expression};
use super::operators::*;
use types::{Array, Text};

pub trait PgExpressionMethods: Expression + Sized {
    /// Creates a PostgreSQL `IS NOT DISTINCT FROM` expression. This behaves
    /// identically to the `=` operator, except that `NULL` is treated as a
    /// normal value.
    ///
    /// # Example
    ///
    /// ```rust
    /// # #[macro_use] extern crate diesel;
    /// # include!("../../doctest_setup.rs");
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
    /// let distinct = users.select(id).filter(name.is_distinct_from("Sean"));
    /// let not_distinct = users.select(id).filter(name.is_not_distinct_from("Sean"));
    /// assert_eq!(Ok(2), distinct.first(&connection));
    /// assert_eq!(Ok(1), not_distinct.first(&connection));
    /// # }
    /// ```
    fn is_not_distinct_from<T>(self, other: T) -> IsNotDistinctFrom<Self, T::Expression>
    where
        T: AsExpression<Self::SqlType>,
    {
        IsNotDistinctFrom::new(self, other.as_expression())
    }

    /// Creates a PostgreSQL `IS DISTINCT FROM` expression. This behaves
    /// identically to the `!=` operator, except that `NULL` is treated as a
    /// normal value.
    ///
    /// # Example
    ///
    /// ```rust
    /// # #[macro_use] extern crate diesel;
    /// # include!("../../doctest_setup.rs");
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
    /// let distinct = users.select(id).filter(name.is_distinct_from("Sean"));
    /// let not_distinct = users.select(id).filter(name.is_not_distinct_from("Sean"));
    /// assert_eq!(Ok(2), distinct.first(&connection));
    /// assert_eq!(Ok(1), not_distinct.first(&connection));
    /// # }
    /// ```
    fn is_distinct_from<T>(self, other: T) -> IsDistinctFrom<Self, T::Expression>
    where
        T: AsExpression<Self::SqlType>,
    {
        IsDistinctFrom::new(self, other.as_expression())
    }
}

impl<T: Expression> PgExpressionMethods for T {}

use super::date_and_time::{AtTimeZone, DateTimeLike};
use types::VarChar;

#[doc(hidden)]
pub trait PgTimestampExpressionMethods: Expression + Sized {
    /// Returns a PostgreSQL "AT TIME ZONE" expression
    fn at_time_zone<T>(self, timezone: T) -> AtTimeZone<Self, T::Expression>
    where
        T: AsExpression<VarChar>,
    {
        AtTimeZone::new(self, timezone.as_expression())
    }
}

impl<T: Expression> PgTimestampExpressionMethods for T
where
    T::SqlType: DateTimeLike,
{
}

pub trait ArrayExpressionMethods<ST>: Expression<SqlType = Array<ST>> + Sized {
    /// Compares two arrays for common elements, using the `&&` operator in
    /// the final SQL
    ///
    /// # Example
    ///
    /// ```rust
    /// # #[macro_use] extern crate diesel;
    /// # include!("../../doctest_setup.rs");
    /// # use schema::users;
    /// #
    /// # table! {
    /// #     posts {
    /// #         id -> Integer,
    /// #         tags -> Array<VarChar>,
    /// #     }
    /// # }
    /// #
    /// # #[derive(Insertable)]
    /// # #[table_name="posts"]
    /// # struct NewPost<'a> { tags: Vec<&'a str> }
    /// #
    /// # fn main() {
    /// #     use self::posts::dsl::*;
    /// #     let conn = establish_connection();
    /// #     conn.execute("DROP TABLE IF EXISTS posts").unwrap();
    /// #     conn.execute("CREATE TABLE posts (id SERIAL PRIMARY KEY, tags TEXT[] NOT NULL)").unwrap();
    /// #
    /// diesel::insert_into(posts)
    ///     .values(&vec![
    ///         NewPost { tags: vec!["cool", "awesome"] },
    ///         NewPost { tags: vec!["awesome", "great"] },
    ///         NewPost { tags: vec!["cool", "great"] },
    ///     ])
    ///     .execute(&conn)
    ///     .unwrap();
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
    fn overlaps_with<T>(self, other: T) -> OverlapsWith<Self, T::Expression>
    where
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
    /// # include!("../../doctest_setup.rs");
    /// # use schema::users;
    /// #
    /// # table! {
    /// #     posts {
    /// #         id -> Integer,
    /// #         tags -> Array<VarChar>,
    /// #     }
    /// # }
    /// #
    /// # #[derive(Insertable)]
    /// # #[table_name="posts"]
    /// # struct NewPost<'a> { tags: Vec<&'a str> }
    /// #
    /// # fn main() {
    /// #     use self::posts::dsl::*;
    /// #     let conn = establish_connection();
    /// #     conn.execute("DROP TABLE IF EXISTS posts").unwrap();
    /// #     conn.execute("CREATE TABLE posts (id SERIAL PRIMARY KEY, tags TEXT[] NOT NULL)").unwrap();
    /// #
    /// diesel::insert_into(posts)
    ///     .values(&vec![
    ///         NewPost { tags: vec!["cool", "awesome"] },
    ///     ])
    ///     .execute(&conn)
    ///     .unwrap();
    ///
    /// let query = posts.select(id).filter(tags.contains(vec!["cool"]));
    /// assert_eq!(Ok(vec![1]), query.load(&conn));
    ///
    /// let query = posts.select(id).filter(tags.contains(vec!["cool", "amazing"]));
    /// assert_eq!(Ok(Vec::new()), query.load::<i32>(&conn));
    /// # }
    /// ```
    fn contains<T>(self, other: T) -> Contains<Self, T::Expression>
    where
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
    /// # include!("../../doctest_setup.rs");
    /// # use schema::users;
    /// #
    /// # table! {
    /// #     posts {
    /// #         id -> Integer,
    /// #         tags -> Array<VarChar>,
    /// #     }
    /// # }
    /// #
    /// # #[derive(Insertable)]
    /// # #[table_name="posts"]
    /// # struct NewPost<'a> { tags: Vec<&'a str> }
    /// #
    /// # fn main() {
    /// #     use self::posts::dsl::*;
    /// #     let conn = establish_connection();
    /// #     conn.execute("DROP TABLE IF EXISTS posts").unwrap();
    /// #     conn.execute("CREATE TABLE posts (id SERIAL PRIMARY KEY, tags TEXT[] NOT NULL)").unwrap();
    /// #
    /// diesel::insert_into(posts)
    ///     .values(&vec![
    ///         NewPost { tags: vec!["cool", "awesome"] },
    ///     ])
    ///     .execute(&conn)
    ///     .unwrap();
    ///
    /// let query = posts.select(id).filter(tags.is_contained_by(vec!["cool", "awesome", "amazing"]));
    /// assert_eq!(Ok(vec![1]), query.load(&conn));
    ///
    /// let query = posts.select(id).filter(tags.is_contained_by(vec!["cool"]));
    /// assert_eq!(Ok(Vec::new()), query.load::<i32>(&conn));
    /// # }
    /// ```
    fn is_contained_by<T>(self, other: T) -> IsContainedBy<Self, T::Expression>
    where
        T: AsExpression<Self::SqlType>,
    {
        IsContainedBy::new(self, other.as_expression())
    }
}

impl<T, ST> ArrayExpressionMethods<ST> for T
where
    T: Expression<SqlType = Array<ST>>,
{
}

use expression::operators::{Asc, Desc};

pub trait SortExpressionMethods: Sized {
    /// Specify that nulls should come before other values in this ordering.
    /// Normally, nulls come last when sorting in ascending order and first
    /// when sorting in descending order.
    ///
    /// # Example
    ///
    /// ```rust
    /// # #[macro_use] extern crate diesel;
    /// # include!("../../doctest_setup.rs");
    /// #
    /// # table! {
    /// #     users {
    /// #         id -> Integer,
    /// #         name -> VarChar,
    /// #     }
    /// # }
    /// #
    /// # table! {
    /// #     foos {
    /// #         id -> Integer,
    /// #         foo -> Nullable<Integer>,
    /// #     }
    /// # }
    /// #
    /// # fn main() {
    /// #     let connection = connection_no_data();
    /// #     connection.execute("DROP TABLE IF EXISTS foos").unwrap();
    /// connection.execute("CREATE TABLE foos (id SERIAL PRIMARY KEY, foo INTEGER)").unwrap();
    /// connection.execute("INSERT INTO foos (foo) VALUES (NULL), (1), (2)").unwrap();
    ///
    /// #     use self::foos::dsl::*;
    /// assert_eq!(Ok(vec![Some(1), Some(2), None]),
    ///            foos.select(foo).order(foo.asc()).load(&connection));
    /// assert_eq!(Ok(vec![None, Some(1), Some(2)]),
    ///            foos.select(foo).order(foo.asc().nulls_first()).load(&connection));
    /// #     connection.execute("DROP TABLE foos").unwrap();
    /// # }
    /// ```
    fn nulls_first(self) -> NullsFirst<Self> {
        NullsFirst::new(self)
    }

    /// Specify that nulls should come after other values in this ordering.
    /// Normally, nulls come last when sorting in ascending order and first
    /// when sorting in descending order.
    ///
    /// # Example
    ///
    /// ```rust
    /// # #[macro_use] extern crate diesel;
    /// # include!("../../doctest_setup.rs");
    /// #
    /// # table! {
    /// #     users {
    /// #         id -> Integer,
    /// #         name -> VarChar,
    /// #     }
    /// # }
    /// #
    /// # table! {
    /// #     foos {
    /// #         id -> Integer,
    /// #         foo -> Nullable<Integer>,
    /// #     }
    /// # }
    /// #
    /// # fn main() {
    /// #     let connection = connection_no_data();
    /// #     connection.execute("DROP TABLE IF EXISTS foos").unwrap();
    /// connection.execute("CREATE TABLE foos (id SERIAL PRIMARY KEY, foo INTEGER)").unwrap();
    /// connection.execute("INSERT INTO foos (foo) VALUES (NULL), (1), (2)").unwrap();
    ///
    /// #     use self::foos::dsl::*;
    /// assert_eq!(Ok(vec![None, Some(2), Some(1)]),
    ///            foos.select(foo).order(foo.desc()).load(&connection));
    /// assert_eq!(Ok(vec![Some(2), Some(1), None]),
    ///            foos.select(foo).order(foo.desc().nulls_last()).load(&connection));
    /// #     connection.execute("DROP TABLE foos").unwrap();
    /// # }
    /// ```
    fn nulls_last(self) -> NullsLast<Self> {
        NullsLast::new(self)
    }
}

impl<T> SortExpressionMethods for Asc<T> {}

impl<T> SortExpressionMethods for Desc<T> {}

pub trait PgTextExpressionMethods: Expression<SqlType = Text> + Sized {
    /// Returns a SQL `ILIKE` expression
    ///
    /// # Example
    ///
    /// ```rust
    /// # #[macro_use] extern crate diesel;
    /// # #[macro_use] extern crate diesel_codegen;
    /// # include!("../../doctest_setup.rs");
    /// # use schema::users;
    /// #
    /// # fn main() {
    /// #     use schema::users::dsl::*;
    /// #     let connection = establish_connection();
    /// #
    /// let like_sean = users
    ///     .select(name)
    ///     .filter(name.ilike("sean"))
    ///     .get_results::<String>(&connection)
    ///     .expect("Failed");
    ///
    /// let expected = vec!["Sean".to_string()];
    ///
    /// assert_eq!(expected, like_sean);
    /// # }
    /// ```
    fn ilike<T: AsExpression<Text>>(self, other: T) -> ILike<Self, T::Expression> {
        ILike::new(self.as_expression(), other.as_expression())
    }

    /// Returns a SQL `NOT ILIKE` expression
    ///
    /// # Example
    ///
    /// ```rust
    /// # #[macro_use] extern crate diesel;
    /// # #[macro_use] extern crate diesel_codegen;
    /// # include!("../../doctest_setup.rs");
    /// # use schema::users;
    /// #
    /// # fn main() {
    /// #     use schema::users::dsl::*;
    /// #     let connection = establish_connection();
    /// #
    /// let not_like_sean = users
    ///     .select(name)
    ///     .filter(name.not_ilike("sean"))
    ///     .get_results::<String>(&connection)
    ///     .expect("Failed");
    ///
    /// let expected = vec!["Tess".to_string()];
    ///
    /// assert_eq!(expected, not_like_sean);
    /// # }
    /// ```
    fn not_ilike<T: AsExpression<Text>>(self, other: T) -> NotILike<Self, T::Expression> {
        NotILike::new(self.as_expression(), other.as_expression())
    }
}

impl<T: Expression<SqlType = Text>> PgTextExpressionMethods for T {}
