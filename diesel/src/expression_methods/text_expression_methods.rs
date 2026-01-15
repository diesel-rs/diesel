use self::private::TextOrNullableText;
use crate::dsl;
use crate::expression::grouped::Grouped;
use crate::expression::operators::{Collate, Concat, Like, NotLike};
use crate::expression::{AsExpression, Expression};

use crate::sql_types::SqlType;

/// Methods present on text expressions
pub trait TextExpressionMethods: Expression + Sized {
    /// Concatenates two strings using the `||` operator.
    ///
    /// # Example
    ///
    /// ```rust
    /// # include!("../doctest_setup.rs");
    /// #
    /// # table! {
    /// #     users {
    /// #         id -> Integer,
    /// #         name -> VarChar,
    /// #         hair_color -> Nullable<Text>,
    /// #     }
    /// # }
    /// #
    /// # fn main() {
    /// #     use self::users::dsl::*;
    /// #     use diesel::insert_into;
    /// #
    /// #     let connection = &mut connection_no_data();
    /// #     diesel::sql_query("CREATE TEMPORARY TABLE users (
    /// #         id INTEGER PRIMARY KEY,
    /// #         name VARCHAR(255) NOT NULL,
    /// #         hair_color VARCHAR(255)
    /// #     )").execute(connection).unwrap();
    /// #
    /// #     insert_into(users)
    /// #         .values(&vec![
    /// #             (id.eq(1), name.eq("Sean"), hair_color.eq(Some("Green"))),
    /// #             (id.eq(2), name.eq("Tess"), hair_color.eq(None)),
    /// #         ])
    /// #         .execute(connection)
    /// #         .unwrap();
    /// #
    /// let names = users.select(name.concat(" the Greatest")).load(connection);
    /// let expected_names = vec![
    ///     "Sean the Greatest".to_string(),
    ///     "Tess the Greatest".to_string(),
    /// ];
    /// assert_eq!(Ok(expected_names), names);
    ///
    /// // If the value is nullable, the output will be nullable
    /// let names = users.select(hair_color.concat("ish")).load(connection);
    /// let expected_names = vec![Some("Greenish".to_string()), None];
    /// assert_eq!(Ok(expected_names), names);
    /// # }
    /// ```
    fn concat<T>(self, other: T) -> dsl::Concat<Self, T>
    where
        Self::SqlType: SqlType,
        T: AsExpression<Self::SqlType>,
    {
        Grouped(Concat::new(self, other.as_expression()))
    }

    /// Returns a SQL `COLLATE` expression.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # include!("../doctest_setup.rs");
    /// #
    /// # fn main() {
    /// #     run_test().unwrap();
    /// # }
    /// #
    /// # fn run_test() -> QueryResult<()> {
    /// #     use schema::users::dsl::*;
    /// #     let connection = &mut establish_connection();
    /// #     #[cfg(feature = "sqlite")]
    /// #     let collation = "BINARY";
    /// #     #[cfg(feature = "postgres")]
    /// #     let collation = "\"C\"";
    /// #     #[cfg(feature = "mysql")]
    /// #     let collation = "utf8mb4_bin";
    /// #
    /// let starts_with_s = users
    ///     .select(name)
    ///     .filter(name.collate(collation).like("S%"))
    ///     .load::<String>(connection)?;
    /// assert_eq!(vec!["Sean"], starts_with_s);
    /// #     Ok(())
    /// # }
    /// ```
    fn collate<S: ToString>(self, collation: S) -> dsl::Collate<Self> {
        Grouped(Collate::new(self, collation))
    }

    /// Returns a SQL `COLLATE BINARY` expression.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # include!("../doctest_setup.rs");
    /// #
    /// # fn main() {
    /// #     run_test().unwrap();
    /// # }
    /// #
    /// # fn run_test() -> QueryResult<()> {
    /// #     use schema::users::dsl::*;
    /// #     let connection = &mut establish_connection();
    /// #     #[cfg(not(feature = "sqlite"))]
    /// #     return Ok(());
    /// #
    /// let starts_with_s = users
    ///     .select(name)
    ///     .filter(name.collate_binary().like("S%"))
    ///     .load::<String>(connection)?;
    /// assert_eq!(vec!["Sean"], starts_with_s);
    /// #     Ok(())
    /// # }
    /// ```
    #[cfg(feature = "sqlite")]
    fn collate_binary(self) -> dsl::Collate<Self> {
        Grouped(Collate::new(self, crate::collation::Binary))
    }

    /// Returns a SQL `COLLATE NOCASE` expression.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # include!("../doctest_setup.rs");
    /// #
    /// # fn main() {
    /// #     run_test().unwrap();
    /// # }
    /// #
    /// # fn run_test() -> QueryResult<()> {
    /// #     use schema::users::dsl::*;
    /// #     let connection = &mut establish_connection();
    /// #     #[cfg(not(feature = "sqlite"))]
    /// #     return Ok(());
    /// #
    /// let starts_with_s = users
    ///     .select(name)
    ///     .filter(name.collate_nocase().eq("sean"))
    ///     .load::<String>(connection)?;
    /// assert_eq!(vec!["Sean"], starts_with_s);
    /// #     Ok(())
    /// # }
    /// ```
    #[cfg(feature = "sqlite")]
    fn collate_nocase(self) -> dsl::Collate<Self> {
        Grouped(Collate::new(self, crate::collation::NoCase))
    }

    /// Returns a SQL `COLLATE RTRIM` expression.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # include!("../doctest_setup.rs");
    /// #
    /// # fn main() {
    /// #     run_test().unwrap();
    /// # }
    /// #
    /// # fn run_test() -> QueryResult<()> {
    /// #     use schema::users::dsl::*;
    /// #     let connection = &mut establish_connection();
    /// #     #[cfg(not(feature = "sqlite"))]
    /// #     return Ok(());
    /// #
    ///     use diesel::insert_into;
    ///     insert_into(users)
    ///        .values(name.eq("Dan   "))
    ///        .execute(connection)?;
    ///
    ///     let names = users
    ///         .select(name)
    ///         .filter(name.collate_rtrim().eq("Dan"))
    ///         .load::<String>(connection)?;
    ///     assert_eq!(vec!["Dan   "], names);
    ///     Ok(())
    /// }
    /// ```
    #[cfg(feature = "sqlite")]
    fn collate_rtrim(self) -> dsl::Collate<Self> {
        Grouped(Collate::new(self, crate::collation::RTrim))
    }

    /// Returns a SQL `COLLATE POSIX` expression.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # include!("../doctest_setup.rs");
    /// #
    /// # fn main() {
    /// #     run_test().unwrap();
    /// # }
    /// #
    /// # fn run_test() -> QueryResult<()> {
    /// #     use schema::users::dsl::*;
    /// #     let connection = &mut establish_connection();
    /// #     #[cfg(not(feature = "postgres"))]
    /// #     return Ok(());
    /// #
    /// let starts_with_s = users
    ///     .select(name)
    ///     .filter(name.collate_posix().eq("Sean"))
    ///     .load::<String>(connection)?;
    /// assert_eq!(vec!["Sean"], starts_with_s);
    /// #     Ok(())
    /// # }
    /// ```
    #[cfg(feature = "postgres")]
    fn collate_posix(self) -> dsl::Collate<Self> {
        Grouped(Collate::new(self, crate::collation::Posix))
    }

    /// Returns a SQL `COLLATE "C"` expression.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # include!("../doctest_setup.rs");
    /// #
    /// # fn main() {
    /// #     run_test().unwrap();
    /// # }
    /// #
    /// # fn run_test() -> QueryResult<()> {
    /// #     use schema::users::dsl::*;
    /// #     let connection = &mut establish_connection();
    /// #     #[cfg(not(feature = "postgres"))]
    /// #     return Ok(());
    /// #
    /// let starts_with_s = users
    ///     .select(name)
    ///     .filter(name.collate_c().eq("Sean"))
    ///     .load::<String>(connection)?;
    /// assert_eq!(vec!["Sean"], starts_with_s);
    /// #     Ok(())
    /// # }
    /// ```
    #[cfg(feature = "postgres")]
    fn collate_c(self) -> dsl::Collate<Self> {
        Grouped(Collate::new(self, crate::collation::C))
    }

    /// Returns a SQL `COLLATE unicode` expression.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # include!("../doctest_setup.rs");
    /// #
    /// # fn main() {
    /// #     run_test().unwrap();
    /// # }
    /// #
    /// # fn run_test() -> QueryResult<()> {
    /// #     use schema::users::dsl::*;
    /// #     let connection = &mut establish_connection();
    /// #     #[cfg(not(feature = "postgres"))]
    /// #     return Ok(());
    /// #
    /// let starts_with_s = users
    ///     .select(name)
    ///     .filter(name.collate_unicode().eq("Sean"))
    ///     .load::<String>(connection)?;
    /// assert_eq!(vec!["Sean"], starts_with_s);
    /// #     Ok(())
    /// # }
    /// ```
    #[cfg(feature = "postgres")]
    fn collate_unicode(self) -> dsl::Collate<Self> {
        Grouped(Collate::new(self, crate::collation::Unicode))
    }

    /// Returns a SQL `COLLATE ucs_basic` expression.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # include!("../doctest_setup.rs");
    /// #
    /// # fn main() {
    /// #     run_test().unwrap();
    /// # }
    /// #
    /// # fn run_test() -> QueryResult<()> {
    /// #     use schema::users::dsl::*;
    /// #     let connection = &mut establish_connection();
    /// #     #[cfg(not(feature = "postgres"))]
    /// #     return Ok(());
    /// #
    /// let starts_with_s = users
    ///     .select(name)
    ///     .filter(name.collate_ucs_basic().eq("Sean"))
    ///     .load::<String>(connection)?;
    /// assert_eq!(vec!["Sean"], starts_with_s);
    /// #     Ok(())
    /// # }
    /// ```
    #[cfg(feature = "postgres")]
    fn collate_ucs_basic(self) -> dsl::Collate<Self> {
        Grouped(Collate::new(self, crate::collation::UcsBasic))
    }

    /// Returns a SQL `COLLATE pg_unicode_fast` expression.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # include!("../doctest_setup.rs");
    /// #
    /// # fn main() {
    /// #     run_test().unwrap();
    /// # }
    /// #
    /// # fn run_test() -> QueryResult<()> {
    /// #     use schema::users::dsl::*;
    /// #     let connection = &mut establish_connection();
    /// #     #[cfg(not(feature = "postgres"))]
    /// #     return Ok(());
    /// #
    /// let starts_with_s = users
    ///     .select(name)
    ///     .filter(name.collate_pg_unicode_fast().eq("Sean"))
    ///     .load::<String>(connection)?;
    /// assert_eq!(vec!["Sean"], starts_with_s);
    /// #     Ok(())
    /// # }
    /// ```
    #[cfg(feature = "postgres")]
    fn collate_pg_unicode_fast(self) -> dsl::Collate<Self> {
        Grouped(Collate::new(self, crate::collation::PgUnicodeFast))
    }

    /// Returns a SQL `COLLATE pg_c_utf8` expression.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # include!("../doctest_setup.rs");
    /// #
    /// # fn main() {
    /// #     run_test().unwrap();
    /// # }
    /// #
    /// # fn run_test() -> QueryResult<()> {
    /// #     use schema::users::dsl::*;
    /// #     let connection = &mut establish_connection();
    /// #     #[cfg(not(feature = "postgres"))]
    /// #     return Ok(());
    /// #
    /// let starts_with_s = users
    ///     .select(name)
    ///     .filter(name.collate_pg_c_utf8().eq("Sean"))
    ///     .load::<String>(connection)?;
    /// assert_eq!(vec!["Sean"], starts_with_s);
    /// #     Ok(())
    /// # }
    /// ```
    #[cfg(feature = "postgres")]
    fn collate_pg_c_utf8(self) -> dsl::Collate<Self> {
        Grouped(Collate::new(self, crate::collation::PgCUtf8))
    }

    /// Returns a SQL `COLLATE default` expression.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # include!("../doctest_setup.rs");
    /// #
    /// # fn main() {
    /// #     run_test().unwrap();
    /// # }
    /// #
    /// # fn run_test() -> QueryResult<()> {
    /// #     use schema::users::dsl::*;
    /// #     let connection = &mut establish_connection();
    /// #     #[cfg(not(feature = "postgres"))]
    /// #     return Ok(());
    /// #
    /// let starts_with_s = users
    ///     .select(name)
    ///     .filter(name.collate_default().eq("Sean"))
    ///     .load::<String>(connection)?;
    /// assert_eq!(vec!["Sean"], starts_with_s);
    /// #     Ok(())
    /// # }
    /// ```
    #[cfg(feature = "postgres")]
    fn collate_default(self) -> dsl::Collate<Self> {
        Grouped(Collate::new(self, crate::collation::Default))
    }

    /// Returns a SQL `LIKE` expression
    ///
    /// This method is case insensitive for SQLite and MySQL.
    /// On PostgreSQL, `LIKE` is case sensitive. You may use
    /// [`ilike()`](../expression_methods/trait.PgTextExpressionMethods.html#method.ilike)
    /// for case insensitive comparison on PostgreSQL.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # include!("../doctest_setup.rs");
    /// #
    /// # fn main() {
    /// #     run_test().unwrap();
    /// # }
    /// #
    /// # fn run_test() -> QueryResult<()> {
    /// #     use schema::users::dsl::*;
    /// #     let connection = &mut establish_connection();
    /// #
    /// let starts_with_s = users
    ///     .select(name)
    ///     .filter(name.like("S%"))
    ///     .load::<String>(connection)?;
    /// assert_eq!(vec!["Sean"], starts_with_s);
    /// #     Ok(())
    /// # }
    /// ```
    fn like<T>(self, other: T) -> dsl::Like<Self, T>
    where
        Self::SqlType: SqlType,
        T: AsExpression<Self::SqlType>,
    {
        Grouped(Like::new(self, other.as_expression()))
    }

    /// Returns a SQL `NOT LIKE` expression
    ///
    /// This method is case insensitive for SQLite and MySQL.
    /// On PostgreSQL `NOT LIKE` is case sensitive. You may use
    /// [`not_ilike()`](../expression_methods/trait.PgTextExpressionMethods.html#method.not_ilike)
    /// for case insensitive comparison on PostgreSQL.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # include!("../doctest_setup.rs");
    /// #
    /// # fn main() {
    /// #     run_test().unwrap();
    /// # }
    /// #
    /// # fn run_test() -> QueryResult<()> {
    /// #     use schema::users::dsl::*;
    /// #     let connection = &mut establish_connection();
    /// #
    /// let doesnt_start_with_s = users
    ///     .select(name)
    ///     .filter(name.not_like("S%"))
    ///     .load::<String>(connection)?;
    /// assert_eq!(vec!["Tess"], doesnt_start_with_s);
    /// #     Ok(())
    /// # }
    /// ```
    fn not_like<T>(self, other: T) -> dsl::NotLike<Self, T>
    where
        Self::SqlType: SqlType,
        T: AsExpression<Self::SqlType>,
    {
        Grouped(NotLike::new(self, other.as_expression()))
    }
}

impl<T> TextExpressionMethods for T
where
    T: Expression,
    T::SqlType: TextOrNullableText,
{
}

mod private {
    use crate::sql_types::{Nullable, Text};

    /// Marker trait used to implement `TextExpressionMethods` on the appropriate
    /// types. Once coherence takes associated types into account, we can remove
    /// this trait.
    pub trait TextOrNullableText {}

    impl TextOrNullableText for Text {}
    impl TextOrNullableText for Nullable<Text> {}

    #[cfg(feature = "postgres_backend")]
    impl TextOrNullableText for crate::pg::sql_types::Citext {}
    #[cfg(feature = "postgres_backend")]
    impl TextOrNullableText for Nullable<crate::pg::sql_types::Citext> {}
}
