use crate::dsl;
use crate::expression::array_comparison::{AsInExpression, In, NotIn};
use crate::expression::grouped::Grouped;
use crate::expression::operators::*;
use crate::expression::{nullable, AsExpression, Expression};
use crate::sql_types::{SingleValue, SqlType};

/// Methods present on all expressions, except tuples
pub trait ExpressionMethods: Expression + Sized {
    /// Creates a SQL `=` expression.
    ///
    /// # Example
    ///
    /// ```rust
    /// # include!("../doctest_setup.rs");
    /// #
    /// # fn main() {
    /// #     use schema::users::dsl::*;
    /// #     let connection = &mut establish_connection();
    /// let data = users.select(id).filter(name.eq("Sean"));
    /// assert_eq!(Ok(1), data.first(connection));
    /// # }
    /// ```
    #[doc(alias = "=")]
    fn eq<T>(self, other: T) -> dsl::Eq<Self, T>
    where
        Self::SqlType: SqlType,
        T: AsExpression<Self::SqlType>,
    {
        Grouped(Eq::new(self, other.as_expression()))
    }

    /// Creates a SQL `!=` expression.
    ///
    /// # Example
    ///
    /// ```rust
    /// # include!("../doctest_setup.rs");
    /// #
    /// # fn main() {
    /// #     use schema::users::dsl::*;
    /// #     let connection = &mut establish_connection();
    /// let data = users.select(id).filter(name.ne("Sean"));
    /// assert_eq!(Ok(2), data.first(connection));
    /// # }
    /// ```
    #[doc(alias = "<>")]
    fn ne<T>(self, other: T) -> dsl::NotEq<Self, T>
    where
        Self::SqlType: SqlType,
        T: AsExpression<Self::SqlType>,
    {
        Grouped(NotEq::new(self, other.as_expression()))
    }

    /// Creates a SQL `IN` statement.
    ///
    /// Queries using this method will not typically be
    /// placed in the prepared statement cache. However,
    /// in cases when a subquery is passed to the method, that
    /// query will use the cache (assuming the subquery
    /// itself is safe to cache).
    /// On PostgreSQL, you should use
    /// `eq(any())` instead. This method may change in the future to
    /// automatically perform `= ANY` on PostgreSQL.
    ///
    /// # Example
    ///
    /// ```rust
    /// # include!("../doctest_setup.rs");
    /// #
    /// # fn main() {
    /// #     use schema::users;
    /// #     use schema::posts;
    /// #     let connection = &mut establish_connection();
    /// #     connection.execute("INSERT INTO users (name) VALUES
    /// #         ('Jim')").unwrap();
    /// let data = users::table.select(users::id).filter(users::name.eq_any(vec!["Sean", "Jim"]));
    /// assert_eq!(Ok(vec![1, 3]), data.load(connection));
    ///
    /// // Calling `eq_any` with an empty array is the same as doing `WHERE 1=0`
    /// let data = users::table.select(users::id).filter(users::name.eq_any(Vec::<String>::new()));
    /// assert_eq!(Ok(vec![]), data.load::<i32>(connection));
    ///
    /// // Calling `eq_any` with a subquery is the same as using
    /// // `WHERE {column} IN {subquery}`.
    ///
    /// let subquery = users::table.filter(users::name.eq("Sean")).select(users::id).into_boxed();
    /// let data = posts::table.select(posts::id).filter(posts::user_id.eq_any(subquery));
    /// assert_eq!(Ok(vec![1, 2]), data.load::<i32>(connection));
    ///
    /// # }
    /// ```
    #[doc(alias = "in")]
    fn eq_any<T>(self, values: T) -> dsl::EqAny<Self, T>
    where
        Self::SqlType: SqlType,
        T: AsInExpression<Self::SqlType>,
    {
        Grouped(In::new(self, values.as_in_expression()))
    }

    /// Creates a SQL `NOT IN` statement.
    ///
    /// Queries using this method will not be
    /// placed in the prepared statement cache. On PostgreSQL, you should use
    /// `ne(all())` instead. This method may change in the future to
    /// automatically perform `!= ALL` on PostgreSQL.
    ///
    /// # Example
    ///
    /// ```rust
    /// # include!("../doctest_setup.rs");
    /// #
    /// # fn main() {
    /// #     use schema::users::dsl::*;
    /// #     let connection = &mut establish_connection();
    /// #     connection.execute("INSERT INTO users (name) VALUES
    /// #         ('Jim')").unwrap();
    /// let data = users.select(id).filter(name.ne_all(vec!["Sean", "Jim"]));
    /// assert_eq!(Ok(vec![2]), data.load(connection));
    ///
    /// let data = users.select(id).filter(name.ne_all(vec!["Tess"]));
    /// assert_eq!(Ok(vec![1, 3]), data.load(connection));
    ///
    /// // Calling `ne_any` with an empty array is the same as doing `WHERE 1=1`
    /// let data = users.select(id).filter(name.ne_all(Vec::<String>::new()));
    /// assert_eq!(Ok(vec![1, 2, 3]), data.load(connection));
    /// # }
    /// ```
    #[doc(alias = "in")]
    fn ne_all<T>(self, values: T) -> dsl::NeAny<Self, T>
    where
        Self::SqlType: SqlType,
        T: AsInExpression<Self::SqlType>,
    {
        Grouped(NotIn::new(self, values.as_in_expression()))
    }

    /// Creates a SQL `IS NULL` expression.
    ///
    /// # Example
    ///
    /// ```rust
    /// # include!("../doctest_setup.rs");
    /// #
    /// # fn main() {
    /// #     run_test().unwrap();
    /// # }
    /// #
    /// # fn run_test() -> QueryResult<()> {
    /// #     use schema::animals::dsl::*;
    /// #     let connection = &mut establish_connection();
    /// #
    /// let data = animals
    ///     .select(species)
    ///     .filter(name.is_null())
    ///     .first::<String>(connection)?;
    /// assert_eq!("spider", data);
    /// #     Ok(())
    /// # }
    fn is_null(self) -> dsl::IsNull<Self> {
        Grouped(IsNull::new(self))
    }

    /// Creates a SQL `IS NOT NULL` expression.
    ///
    /// # Example
    ///
    /// ```rust
    /// # include!("../doctest_setup.rs");
    /// #
    /// # fn main() {
    /// #     run_test().unwrap();
    /// # }
    /// #
    /// # fn run_test() -> QueryResult<()> {
    /// #     use schema::animals::dsl::*;
    /// #     let connection = &mut establish_connection();
    /// #
    /// let data = animals
    ///     .select(species)
    ///     .filter(name.is_not_null())
    ///     .first::<String>(connection)?;
    /// assert_eq!("dog", data);
    /// #     Ok(())
    /// # }
    fn is_not_null(self) -> dsl::IsNotNull<Self> {
        Grouped(IsNotNull::new(self))
    }

    /// Creates a SQL `>` expression.
    ///
    /// # Example
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
    /// let data = users
    ///     .select(name)
    ///     .filter(id.gt(1))
    ///     .first::<String>(connection)?;
    /// assert_eq!("Tess", data);
    /// #     Ok(())
    /// # }
    /// ```
    #[doc(alias = ">")]
    fn gt<T>(self, other: T) -> dsl::Gt<Self, T>
    where
        Self::SqlType: SqlType,
        T: AsExpression<Self::SqlType>,
    {
        Grouped(Gt::new(self, other.as_expression()))
    }

    /// Creates a SQL `>=` expression.
    ///
    /// # Example
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
    /// let data = users
    ///     .select(name)
    ///     .filter(id.ge(2))
    ///     .first::<String>(connection)?;
    /// assert_eq!("Tess", data);
    /// #     Ok(())
    /// # }
    /// ```
    #[doc(alias = ">=")]
    fn ge<T>(self, other: T) -> dsl::GtEq<Self, T>
    where
        Self::SqlType: SqlType,
        T: AsExpression<Self::SqlType>,
    {
        Grouped(GtEq::new(self, other.as_expression()))
    }

    /// Creates a SQL `<` expression.
    ///
    /// # Example
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
    /// let data = users
    ///     .select(name)
    ///     .filter(id.lt(2))
    ///     .first::<String>(connection)?;
    /// assert_eq!("Sean", data);
    /// #     Ok(())
    /// # }
    /// ```
    #[doc(alias = "<")]
    fn lt<T>(self, other: T) -> dsl::Lt<Self, T>
    where
        Self::SqlType: SqlType,
        T: AsExpression<Self::SqlType>,
    {
        Grouped(Lt::new(self, other.as_expression()))
    }

    /// Creates a SQL `<=` expression.
    ///
    /// # Example
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
    /// let data = users
    ///     .select(name)
    ///     .filter(id.le(2))
    ///     .first::<String>(connection)?;
    /// assert_eq!("Sean", data);
    /// #     Ok(())
    /// # }
    #[doc(alias = "<=")]
    fn le<T>(self, other: T) -> dsl::LtEq<Self, T>
    where
        Self::SqlType: SqlType,
        T: AsExpression<Self::SqlType>,
    {
        Grouped(LtEq::new(self, other.as_expression()))
    }

    /// Creates a SQL `BETWEEN` expression using the given lower and upper
    /// bounds.
    ///
    /// # Example
    ///
    /// ```rust
    /// # include!("../doctest_setup.rs");
    /// #
    /// # fn main() {
    /// #     use schema::animals::dsl::*;
    /// #     let connection = &mut establish_connection();
    /// #
    /// let data = animals
    ///     .select(species)
    ///     .filter(legs.between(2, 6))
    ///     .first(connection);
    /// #
    /// assert_eq!(Ok("dog".to_string()), data);
    /// # }
    /// ```
    fn between<T, U>(self, lower: T, upper: U) -> dsl::Between<Self, T, U>
    where
        Self::SqlType: SqlType,
        T: AsExpression<Self::SqlType>,
        U: AsExpression<Self::SqlType>,
    {
        Grouped(Between::new(
            self,
            And::new(lower.as_expression(), upper.as_expression()),
        ))
    }

    /// Creates a SQL `NOT BETWEEN` expression using the given lower and upper
    /// bounds.
    ///
    /// # Example
    ///
    /// ```rust
    /// # include!("../doctest_setup.rs");
    /// #
    /// # fn main() {
    /// #     run_test().unwrap();
    /// # }
    /// #
    /// # fn run_test() -> QueryResult<()> {
    /// #     use schema::animals::dsl::*;
    /// #     let connection = &mut establish_connection();
    /// #
    /// let data = animals
    ///     .select(species)
    ///     .filter(legs.not_between(2, 6))
    ///     .first::<String>(connection)?;
    /// assert_eq!("spider", data);
    /// #     Ok(())
    /// # }
    fn not_between<T, U>(self, lower: T, upper: U) -> dsl::NotBetween<Self, T, U>
    where
        Self::SqlType: SqlType,
        T: AsExpression<Self::SqlType>,
        U: AsExpression<Self::SqlType>,
    {
        Grouped(NotBetween::new(
            self,
            And::new(lower.as_expression(), upper.as_expression()),
        ))
    }

    /// Creates a SQL `DESC` expression, representing this expression in
    /// descending order.
    ///
    /// # Example
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
    /// let names = users
    ///     .select(name)
    ///     .order(name.desc())
    ///     .load::<String>(connection)?;
    /// assert_eq!(vec!["Tess", "Sean"], names);
    /// #     Ok(())
    /// # }
    /// ```
    fn desc(self) -> dsl::Desc<Self> {
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
    /// # include!("../doctest_setup.rs");
    /// # use diesel::expression::expression_types::NotSelectable;
    /// #
    /// # fn main() {
    /// #     use schema::users::dsl::*;
    /// #     let order = "name";
    /// let ordering: Box<dyn BoxableExpression<users, DB, SqlType = NotSelectable>> =
    ///     if order == "name" {
    ///         Box::new(name.desc())
    ///     } else {
    ///         Box::new(id.asc())
    ///     };
    /// # }
    /// ```
    fn asc(self) -> dsl::Asc<Self> {
        Asc::new(self)
    }
}

impl<T> ExpressionMethods for T
where
    T: Expression,
    T::SqlType: SingleValue,
{
}

/// Methods present on all expressions
pub trait NullableExpressionMethods: Expression + Sized {
    /// Converts this potentially non-null expression into one which is treated
    /// as nullable. This method has no impact on the generated SQL, and is only
    /// used to allow certain comparisons that would otherwise fail to compile.
    ///
    /// # Example
    /// ```no_run
    /// # #![allow(dead_code)]
    /// # include!("../doctest_setup.rs");
    /// # use diesel::sql_types::*;
    /// # use schema::users;
    /// #
    /// table! {
    ///     posts {
    ///         id -> Integer,
    ///         user_id -> Integer,
    ///         author_name -> Nullable<VarChar>,
    ///     }
    /// }
    /// #
    /// # joinable!(posts -> users (user_id));
    /// # allow_tables_to_appear_in_same_query!(posts, users);
    ///
    /// fn main() {
    ///     use self::users::dsl::*;
    ///     use self::posts::dsl::{posts, author_name};
    ///     let connection = &mut establish_connection();
    ///
    ///     let data = users.inner_join(posts)
    ///         .filter(name.nullable().eq(author_name))
    ///         .select(name)
    ///         .load::<String>(connection);
    ///     println!("{:?}", data);
    /// }
    /// ```
    fn nullable(self) -> dsl::Nullable<Self> {
        nullable::Nullable::new(self)
    }
}

impl<T: Expression> NullableExpressionMethods for T {}
