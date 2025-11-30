#[cfg(doc)]
use super::aggregate_expressions::{AggregateExpressionMethods, WindowExpressionMethods};
use crate::expression::functions::declare_sql_function;
use crate::sql_types::Foldable;

#[declare_sql_function]
extern "SQL" {
    /// Represents a SQL `SUM` function. This function can only take types which are
    /// Foldable.
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
    /// assert_eq!(Ok(Some(12i64)), animals.select(sum(legs)).first(connection));
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
    ///     .select((name, sum(legs).partition_by(id)))
    ///     .load::<(Option<String>, Option<i64>)>(connection);
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
    ///     Ok(Some(4i64)),
    ///     animals
    ///         .select(sum(legs).aggregate_filter(legs.lt(8)))
    ///         .first(connection)
    /// );
    /// # }
    /// ```
    #[aggregate]
    #[window]
    fn sum<ST: Foldable>(expr: ST) -> ST::Sum;

    /// Represents a SQL `AVG` function. This function can only take types which are
    /// Foldable.
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
    /// # #[cfg(feature = "numeric")]
    /// # extern crate bigdecimal;
    /// #
    /// # fn main() {
    /// #     run_test().unwrap();
    /// # }
    /// #
    /// # table! {
    /// #     numbers (number) {
    /// #         number -> Integer,
    /// #     }
    /// # }
    /// #
    /// # #[cfg(all(feature = "numeric", any(feature = "postgres", not(feature = "sqlite"))))]
    /// # fn run_test() -> QueryResult<()> {
    /// #     use bigdecimal::BigDecimal;
    /// #     use self::numbers::dsl::*;
    /// #     let conn = &mut establish_connection();
    /// #     diesel::sql_query("CREATE TEMPORARY TABLE numbers (number INTEGER)").execute(conn)?;
    /// diesel::insert_into(numbers)
    ///     .values(&vec![number.eq(1), number.eq(2)])
    ///     .execute(conn)?;
    /// let average = numbers.select(avg(number)).get_result(conn)?;
    /// let expected = "1.5".parse::<BigDecimal>().unwrap();
    /// assert_eq!(Some(expected), average);
    /// #     Ok(())
    /// # }
    /// #
    /// # #[cfg(not(all(feature = "numeric", any(feature = "postgres", not(feature = "sqlite")))))]
    /// # fn run_test() -> QueryResult<()> {
    /// #     Ok(())
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
    /// #     run_test().unwrap();
    /// # }
    /// #
    /// # #[cfg(not(all(feature = "numeric", feature = "postgres")))]
    /// # fn run_test() -> QueryResult<()> {
    /// #    Ok(())
    /// # }
    /// # #[cfg(all(feature = "numeric", feature = "postgres"))]
    /// fn run_test() -> QueryResult<()> {
    /// #     use schema::animals::dsl::*;
    /// #     use bigdecimal::BigDecimal;
    /// #     let connection = &mut establish_connection();
    /// let res = animals.select((name, avg(legs).partition_by(id))).load::<(Option<String>, Option<BigDecimal>)>(connection)?;
    ///
    /// assert_eq!(vec![
    ///         (Some("Jack".into()), "4".parse::<BigDecimal>().ok()),
    ///         (None, "8".parse::<BigDecimal>().ok()),
    ///     ], res);
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// ## Aggregate function expression
    ///
    /// ```rust
    /// # include!("../../doctest_setup.rs");
    /// # use diesel::dsl::*;
    /// # #[cfg(feature = "numeric")]
    /// # extern crate bigdecimal;
    /// #
    /// # fn main() {
    /// #     run_test().unwrap();
    /// # }
    /// #
    /// # table! {
    /// #     numbers (number) {
    /// #         number -> Integer,
    /// #     }
    /// # }
    /// #
    /// # #[cfg(all(feature = "numeric", feature = "postgres"))]
    /// # fn run_test() -> QueryResult<()> {
    /// #     use bigdecimal::BigDecimal;
    /// #     use self::numbers::dsl::*;
    /// #     let conn = &mut establish_connection();
    /// #     diesel::sql_query("CREATE TEMPORARY TABLE numbers (number INTEGER)").execute(conn)?;
    /// diesel::insert_into(numbers)
    ///     .values(&vec![number.eq(1), number.eq(2), number.eq(3)])
    ///     .execute(conn)?;
    ///
    /// let average = numbers.select(avg(number).aggregate_filter(number.lt(3))).get_result(conn)?;
    /// let expected = "1.5".parse::<BigDecimal>().unwrap();
    /// assert_eq!(Some(expected), average);
    /// #     Ok(())
    /// # }
    /// #
    /// # #[cfg(not(all(feature = "numeric", feature = "postgres")))]
    /// # fn run_test() -> QueryResult<()> {
    /// #     Ok(())
    /// # }
    #[aggregate]
    #[window]
    fn avg<ST: Foldable>(expr: ST) -> ST::Avg;
}
