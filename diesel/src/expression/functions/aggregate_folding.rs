use crate::expression::functions::define_sql_function;
use crate::sql_types::Foldable;

define_sql_function! {
    /// Represents a SQL `SUM` function. This function can only take types which are
    /// Foldable.
    ///
    /// # Examples
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
    #[aggregate]
    fn sum<ST: Foldable>(expr: ST) -> ST::Sum;
}

define_sql_function! {
    /// Represents a SQL `AVG` function. This function can only take types which are
    /// Foldable.
    ///
    /// # Examples
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
    /// #     diesel::sql_query("DROP TABLE IF EXISTS numbers").execute(conn)?;
    /// #     diesel::sql_query("CREATE TABLE numbers (number INTEGER)").execute(conn)?;
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
    #[aggregate]
    fn avg<ST: Foldable>(expr: ST) -> ST::Avg;
}
