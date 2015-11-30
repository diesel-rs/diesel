use expression::{Expression, AsExpression};
use expression::aliased::Aliased;
use expression::predicates::*;
use expression::ordering;

pub trait ExpressionMethods: Expression + Sized {
    /// Alias an expression for use alongside
    /// [`with`](../../../trait.WithDsl.html).
    ///
    /// While you will need to give it a name to alias as, you should not need
    /// to reference the alias elsewhere. You can pass the returned expression
    /// anywhere you want to reference the alias.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let query = plain_to_tsquery(search_text).aliased("q");
    /// let rank = ts_rank(query, indexed_search_column).aliased("rank");
    /// crates.with(query).with(rank)
    ///     .filter(query.matches(indexed_search_column))
    ///     .order(rank.desc())
    /// ```
    fn aliased<'a>(self, alias: &'a str) -> Aliased<'a, Self> {
        Aliased::new(self, alias)
    }

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
    /// #         id -> Serial,
    /// #         name -> VarChar,
    /// #     }
    /// # }
    /// #
    /// # fn main() {
    /// #     use self::users::dsl::*;
    /// #     let connection = establish_connection();
    /// let data = users.select(id).filter(name.eq("Sean"));
    /// assert_eq!(1, data.first(&connection).unwrap());
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
    /// #         id -> Serial,
    /// #         name -> VarChar,
    /// #     }
    /// # }
    /// #
    /// # fn main() {
    /// #     use self::users::dsl::*;
    /// #     let connection = establish_connection();
    /// let data = users.select(id).filter(name.ne("Sean"));
    /// assert_eq!(2, data.first(&connection).unwrap());
    /// # }
    /// ```
    fn ne<T: AsExpression<Self::SqlType>>(self, other: T) -> NotEq<Self, T::Expression> {
        NotEq::new(self, other.as_expression())
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
    /// #         id -> Serial,
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
    /// #         id -> Serial,
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
    /// #         id -> Serial,
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
    /// #         id -> Serial,
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
    fn desc(self) -> ordering::Desc<Self> {
        ordering::Desc::new(self)
    }

    /// Creates a SQL `ASC` expression, representing this expression in
    /// descending order.
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
    /// #         id -> Serial,
    /// #         name -> VarChar,
    /// #     }
    /// # }
    /// #
    /// # fn main() {
    /// #     use self::users::dsl::*;
    /// #     let order = "name";
    /// let ordering: Box<BoxableExpression<users, (), SqlType=()>> =
    ///     if order == "name" {
    ///         Box::new(name.desc())
    ///     } else {
    ///         Box::new(id.asc())
    ///     };
    /// # }
    /// ```
    fn asc(self) -> ordering::Asc<Self> {
        ordering::Asc::new(self)
    }
}

impl<T: Expression> ExpressionMethods for T {}
