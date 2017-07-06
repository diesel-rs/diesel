use expression::{Expression, AsExpression, nullable};
use expression::array_comparison::{In, NotIn, AsInExpression};
use expression::operators::*;
use types::SingleValue;

pub trait ExpressionMethods: Expression + Sized {
    /// Creates a SQL `=` expression.
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
    /// let data = users.select(id).filter(name.eq("Sean"));
    /// assert_eq!(Ok(1), data.first(&connection));
    /// # }
    /// ```
    fn eq<T: AsExpression<Self::SqlType>>(self, other: T) -> Eq<Self, T::Expression> {
        Eq::new(self, other.as_expression())
    }

    /// Creates a SQL `!=` expression.
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
    /// let data = users.select(id).filter(name.ne("Sean"));
    /// assert_eq!(Ok(2), data.first(&connection));
    /// # }
    /// ```
    fn ne<T: AsExpression<Self::SqlType>>(self, other: T) -> NotEq<Self, T::Expression> {
        NotEq::new(self, other.as_expression())
    }

    /// Creates a SQL `IN` statement. Queries using this method will not be
    /// placed in the prepared statement cache. On PostgreSQL, you should use
    /// `eq(any())` instead. This method may change in the future to
    /// automatically perform `= ANY` on PostgreSQL.
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
    /// #     connection.execute("INSERT INTO users (name) VALUES
    /// #         ('Jim')").unwrap();
    /// let data = users.select(id).filter(name.eq_any(vec!["Sean", "Jim"]));
    /// assert_eq!(Ok(vec![1, 3]), data.load(&connection));
    ///
    /// // Calling `eq_any` with an empty array is the same as doing `WHERE 1=0`
    /// let data = users.select(id).filter(name.eq_any(Vec::<String>::new()));
    /// assert_eq!(Ok(vec![]), data.load::<i32>(&connection));
    /// # }
    /// ```
    fn eq_any<T>(self, values: T) -> In<Self, T::InExpression> where
        T: AsInExpression<Self::SqlType>,
    {
        In::new(self, values.as_in_expression())
    }

    /// Creates a SQL `NOT IN` statement. Queries using this method will not be
    /// placed in the prepared statement cache. On PostgreSQL, you should use
    /// `ne(any())` instead. This method may change in the future to
    /// automatically perform `!= ANY` on PostgreSQL.
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
    /// #     connection.execute("INSERT INTO users (name) VALUES
    /// #         ('Jim')").unwrap();
    /// let data = users.select(id).filter(name.ne_any(vec!["Sean", "Jim"]));
    /// assert_eq!(Ok(vec![2]), data.load(&connection));
    ///
    /// let data = users.select(id).filter(name.ne_any(vec!["Tess"]));
    /// assert_eq!(Ok(vec![1, 3]), data.load(&connection));
    ///
    /// // Calling `ne_any` with an empty array is the same as doing `WHERE 1=1`
    /// let data = users.select(id).filter(name.ne_any(Vec::<String>::new()));
    /// assert_eq!(Ok(vec![1, 2, 3]), data.load(&connection));
    /// # }
    /// ```
    fn ne_any<T>(self, values: T) -> NotIn<Self, T::InExpression> where
        T: AsInExpression<Self::SqlType>,
    {
        NotIn::new(self, values.as_in_expression())
    }

    /// Creates a SQL `IS NULL` expression.
    fn is_null(self) -> IsNull<Self> {
       IsNull::new(self)
    }

    /// Creates a SQL `IS NOT NULL` expression.
    fn is_not_null(self) -> IsNotNull<Self> {
       IsNotNull::new(self)
    }

    /// Creates a SQL `>` expression.
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
    /// let data = users.select(name).filter(id.gt(1));
    /// assert_eq!(Ok("Tess".to_string()), data.first(&connection));
    /// # }
    /// ```
    fn gt<T: AsExpression<Self::SqlType>>(self, other: T) -> Gt<Self, T::Expression> {
        Gt::new(self, other.as_expression())
    }

    /// Creates a SQL `>=` expression.
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
    /// let data = users.select(name).filter(id.ge(2));
    /// assert_eq!(Ok("Tess".to_string()), data.first(&connection));
    /// # }
    /// ```
    fn ge<T: AsExpression<Self::SqlType>>(self, other: T) -> GtEq<Self, T::Expression> {
        GtEq::new(self, other.as_expression())
    }

    /// Creates a SQL `<` expression.
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
    /// let data = users.select(name).filter(id.lt(2));
    /// assert_eq!(Ok("Sean".to_string()), data.first(&connection));
    /// # }
    /// ```
    fn lt<T: AsExpression<Self::SqlType>>(self, other: T) -> Lt<Self, T::Expression> {
        Lt::new(self, other.as_expression())
    }

    /// Creates a SQL `<=` expression.
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
    /// let data = users.select(name).filter(id.le(2));
    /// assert_eq!(Ok("Sean".to_string()), data.first(&connection));
    /// # }
    fn le<T: AsExpression<Self::SqlType>>(self, other: T) -> LtEq<Self, T::Expression> {
        LtEq::new(self, other.as_expression())
    }

    /// Creates a SQL `BETWEEN` expression using the given range.
    fn between<T: AsExpression<Self::SqlType>>(self, other: ::std::ops::Range<T>)
    -> Between<Self, And<T::Expression, T::Expression>> {
        Between::new(self, And::new(other.start.as_expression(), other.end.as_expression()))
    }

    /// Creates a SQL `NOT BETWEEN` expression using the given range.
    fn not_between<T: AsExpression<Self::SqlType>>(self, other: ::std::ops::Range<T>)
    -> NotBetween<Self, And<T::Expression, T::Expression>> {
        NotBetween::new(self, And::new(other.start.as_expression(), other.end.as_expression()))
    }

    /// Creates a SQL `DESC` expression, representing this expression in
    /// descending order.
    fn desc(self) -> Desc<Self> {
        Desc::new(self)
    }

    /// Creates a SQL `ASC` expression, representing this expression in
    /// ascending order.
    ///
    /// This is the same as leaving the direction unspecified. It is useful if
    /// you need to provide an unknown ordering, and need to box the return
    /// value of a function.
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
    /// #     let order = "name";
    /// let ordering: Box<BoxableExpression<users, DB, SqlType=()>> =
    ///     if order == "name" {
    ///         Box::new(name.desc())
    ///     } else {
    ///         Box::new(id.asc())
    ///     };
    /// # }
    /// ```
    fn asc(self) -> Asc<Self> {
        Asc::new(self)
    }
}

impl<T> ExpressionMethods for T
where
    T: Expression,
    T::SqlType: SingleValue,
{
}

pub trait NullableExpressionMethods: Expression + Sized {
    /// Converts this potentially non-null expression into one which is treated
    /// as nullable. This method has no impact on the generated SQL, and is only
    /// used to allow certain comparisons that would otherwise fail to compile.
    ///
    /// # Example
    /// ```no_run
    /// # #![allow(dead_code)]
    /// # #[macro_use] extern crate diesel;
    /// # include!("src/doctest_setup.rs");
    /// # use self::diesel::types::*;
    /// #
    /// table! {
    ///     users {
    ///         id -> Integer,
    ///         name -> VarChar,
    ///     }
    /// }
    ///
    /// table! {
    ///     posts {
    ///         id -> Integer,
    ///         user_id -> Integer,
    ///         author_name -> Nullable<VarChar>,
    ///     }
    /// }
    /// #
    /// #  pub struct User {
    /// #      id: i32,
    /// #      name: VarChar,
    /// #  }
    /// #
    /// #  pub struct Post {
    /// #      id: i32,
    /// #      user_id: i32,
    /// #      author_name: Option<VarChar>,
    /// #  }
    /// #
    /// #  joinable!(posts -> users (user_id));
    ///
    /// fn main() {
    ///     use self::users::dsl::*;
    ///     use self::posts::dsl::{posts, author_name};
    ///     let connection = establish_connection();
    ///
    ///     let data = users.inner_join(posts)
    ///         .filter(name.nullable().eq(author_name))
    ///         .select(name)
    ///         .load::<String>(&connection);
    ///     println!("{:?}", data);
    /// }
    fn nullable(self) -> nullable::Nullable<Self> {
        nullable::Nullable::new(self)
    }
}

impl<T: Expression> NullableExpressionMethods for T {}
