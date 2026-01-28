use self::private::SqlOrdAggregate;
#[cfg(doc)]
use super::aggregate_expressions::{AggregateExpressionMethods, WindowExpressionMethods};
use crate::expression::functions::declare_sql_function;

#[declare_sql_function]
extern "SQL" {
    /// Represents a SQL `MAX` function. This function can only take types which are
    /// ordered.
    ///
    /// ## Window Function Usage
    ///
    /// This function can be used as window function. See [`WindowExpressionMethods`] for details
    ///
    /// ## Aggregate Function Expression
    ///
    /// This function can be used as aggregate expression. See [`AggregateExpressionMethods`] for details.
    ///
    /// # Examples
    ///
    /// ## Normal function usage
    ///
    /// ```rust
    /// # include!("../../doctest_setup.rs");
    /// # use diesel::dsl::*;
    /// #
    /// # fn main() {
    /// #     use schema::animals::dsl::*;
    /// #     let connection = &mut establish_connection();
    /// assert_eq!(Ok(Some(8)), animals.select(max(legs)).first(connection));
    /// # }
    /// ```
    ///
    /// ## Window function
    ///
    /// ```rust
    /// # include!("../../doctest_setup.rs");
    /// # use diesel::dsl::*;
    /// #
    /// # fn main() {
    /// #     use schema::animals::dsl::*;
    /// #     let connection = &mut establish_connection();
    /// let res = animals
    ///     .select((name, max(legs).partition_by(id)))
    ///     .load::<(Option<String>, Option<i32>)>(connection);
    ///
    /// assert_eq!(
    ///     Ok(vec![(Some("Jack".into()), Some(4)), (None, Some(8))]),
    ///     res
    /// );
    /// # }
    /// ```
    ///
    ///
    /// ## Aggregate function expression
    ///
    /// ```rust
    /// # include!("../../doctest_setup.rs");
    /// # use diesel::dsl::*;
    /// #
    /// # fn main() {
    /// #     use schema::animals::dsl::*;
    /// #     let connection = &mut establish_connection();
    /// #     #[cfg(not(feature = "mysql"))]
    /// assert_eq!(
    ///     Ok(Some(4)),
    ///     animals
    ///         .select(max(legs).aggregate_filter(legs.lt(8)))
    ///         .first(connection)
    /// );
    /// # }
    /// ```
    #[aggregate]
    #[window]
    fn max<ST: SqlOrdAggregate>(expr: ST) -> ST::Ret;

    /// Represents a SQL `MIN` function. This function can only take types which are
    /// ordered.
    ///
    /// ## Window Function Usage
    ///
    /// This function can be used as window function. See [`WindowExpressionMethods`] for details
    ///
    /// ## Aggregate Function Expression
    ///
    /// This function can be used as aggregate expression. See [`AggregateExpressionMethods`] for details.
    ///
    /// # Examples
    ///
    /// ## Normal function usage
    ///
    /// ```rust
    /// # include!("../../doctest_setup.rs");
    /// # use diesel::dsl::*;
    /// #
    /// # fn main() {
    /// #     use schema::animals::dsl::*;
    /// #     let connection = &mut establish_connection();
    /// assert_eq!(Ok(Some(4)), animals.select(min(legs)).first(connection));
    /// # }
    /// ```
    ///
    /// ## Window function
    ///
    /// ```rust
    /// # include!("../../doctest_setup.rs");
    /// # use diesel::dsl::*;
    /// #
    /// # fn main() {
    /// #     use schema::animals::dsl::*;
    /// #     let connection = &mut establish_connection();
    /// let res = animals
    ///     .select((name, min(legs).partition_by(id)))
    ///     .load::<(Option<String>, Option<i32>)>(connection);
    ///
    /// assert_eq!(
    ///     Ok(vec![(Some("Jack".into()), Some(4)), (None, Some(8))]),
    ///     res
    /// );
    /// # }
    /// ```
    ///
    /// ## Aggregate function expression
    ///
    /// ```rust
    /// # include!("../../doctest_setup.rs");
    /// # use diesel::dsl::*;
    /// #
    /// # fn main() {
    /// #     use schema::animals::dsl::*;
    /// #     let connection = &mut establish_connection();
    /// #     #[cfg(not(feature = "mysql"))]
    /// assert_eq!(
    ///     Ok(Some(8)),
    ///     animals
    ///         .select(min(legs).aggregate_filter(legs.gt(4)))
    ///         .first(connection)
    /// );
    /// # }
    /// ```
    #[aggregate]
    #[window]
    fn min<ST: SqlOrdAggregate>(expr: ST) -> ST::Ret;
}

mod private {
    use crate::sql_types::{IntoNullable, SingleValue, SqlOrd, SqlType};
    pub trait SqlOrdAggregate: SingleValue {
        type Ret: SqlType + SingleValue;
    }

    impl<T> SqlOrdAggregate for T
    where
        T: SqlOrd + IntoNullable + SingleValue,
        T::Nullable: SqlType + SingleValue,
    {
        type Ret = T::Nullable;
    }
}
