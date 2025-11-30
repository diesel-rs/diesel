#[cfg(doc)]
use super::aggregate_expressions::WindowExpressionMethods;
use crate::sql_types::helper::CombinedNullableValue;
use crate::sql_types::{Integer, IntoNotNullable, IntoNullable, SingleValue, SqlType};
use diesel_derives::declare_sql_function;

#[declare_sql_function]
extern "SQL" {

    /// Number of th current row within its partition
    ///
    /// Returns the number of the current row within its partition, counting from 1.
    ///
    /// This function must be used as window function. You need to call at least one
    /// of the methods [`WindowExpressionMethods`] from to use this function in your `SELECT`
    /// clause. It cannot be used outside of `SELECT` clauses.
    ///
    /// ```
    /// # include!("../../doctest_setup.rs");
    /// # use diesel::dsl::*;
    /// #
    /// # fn main() -> QueryResult<()> {
    /// #     use schema::posts::dsl::*;
    /// #     let connection = &mut establish_connection();
    /// let res = posts
    ///     .select((title, user_id, row_number().partition_by(user_id)))
    ///     .load::<(String, i32, i64)>(connection)?;
    /// let expected = vec![
    ///     ("My first post".to_owned(), 1, 1),
    ///     ("About Rust".into(), 1, 2),
    ///     ("My first post too".into(), 2, 1),
    /// ];
    /// assert_eq!(expected, res);
    /// # Ok(())
    /// # }
    /// ```
    #[window]
    fn row_number() -> BigInt;

    /// Rank of current row within its partition, with gaps
    ///
    /// Returns the rank of the current row, with gaps;
    /// that is, the row_number of the first row in its peer group.
    ///
    /// This function must be used as window function. You need to call at least one
    /// of the methods [`WindowExpressionMethods`] from to use this function in your `SELECT`
    /// clause. It cannot be used outside of `SELECT` clauses.
    ///
    /// For MySQL this function requires you to call [`.window_order()`](WindowExpressionMethods::window_order())
    ///
    /// ```
    /// # include!("../../doctest_setup.rs");
    /// # use diesel::dsl::*;
    /// #
    /// # fn main() -> QueryResult<()> {
    /// #     use schema::posts::dsl::*;
    /// #     let connection = &mut establish_connection();
    /// let res = posts
    ///     .select((
    ///         title,
    ///         user_id,
    ///         rank().partition_by(user_id).window_order(user_id),
    ///     ))
    ///     .load::<(String, i32, i64)>(connection)?;
    /// let expected = vec![
    ///     ("My first post".to_owned(), 1, 1),
    ///     ("About Rust".into(), 1, 1),
    ///     ("My first post too".into(), 2, 1),
    /// ];
    /// assert_eq!(expected, res);
    /// # Ok(())
    /// # }
    /// ```
    #[window(dialect(
        BuiltInWindowFunctionRequireOrder,
        crate::backend::sql_dialect::built_in_window_function_require_order::NoOrderRequired
    ))]
    #[cfg_attr(
        feature = "mysql_backend",
        window(backends(diesel::mysql::Mysql), require_order = true)
    )]
    fn rank() -> BigInt;

    /// Rank of current row within its partition, without gaps
    ///
    /// Returns the rank of the current row, without gaps;
    /// this function effectively counts peer groups.
    ///
    /// This function must be used as window function. You need to call at least one
    /// of the methods [`WindowExpressionMethods`] from to use this function in your `SELECT`
    /// clause. It cannot be used outside of `SELECT` clauses.
    ///
    /// For MySQL this function requires you to call [`.window_order()`](WindowExpressionMethods::window_order())
    ///
    /// ```
    /// # include!("../../doctest_setup.rs");
    /// # use diesel::dsl::*;
    /// #
    /// # fn main() -> QueryResult<()> {
    /// #     use schema::posts::dsl::*;
    /// #     let connection = &mut establish_connection();
    /// let res = posts
    ///     .select((
    ///         title,
    ///         user_id,
    ///         dense_rank().partition_by(user_id).window_order(user_id),
    ///     ))
    ///     .load::<(String, i32, i64)>(connection)?;
    /// let expected = vec![
    ///     ("My first post".to_owned(), 1, 1),
    ///     ("About Rust".into(), 1, 1),
    ///     ("My first post too".into(), 2, 1),
    /// ];
    /// assert_eq!(expected, res);
    /// # Ok(())
    /// # }
    /// ```
    #[window(dialect(
        BuiltInWindowFunctionRequireOrder,
        crate::backend::sql_dialect::built_in_window_function_require_order::NoOrderRequired
    ))]
    #[cfg_attr(
        feature = "mysql_backend",
        window(backends(diesel::mysql::Mysql), require_order = true)
    )]
    fn dense_rank() -> BigInt;

    /// Percentage rank value
    ///
    /// Returns the relative rank of the current row,
    /// that is (rank - 1) / (total partition rows - 1).
    /// The value thus ranges from 0 to 1 inclusive.
    ///
    /// This function must be used as window function. You need to call at least one
    /// of the methods [`WindowExpressionMethods`] from to use this function in your `SELECT`
    /// clause. It cannot be used outside of `SELECT` clauses.
    ///
    /// For MySQL this function requires you to call [`.window_order()`](WindowExpressionMethods::window_order())
    ///
    /// ```
    /// # include!("../../doctest_setup.rs");
    /// # use diesel::dsl::*;
    /// #
    /// # fn main() -> QueryResult<()> {
    /// #     use schema::posts::dsl::*;
    /// #     let connection = &mut establish_connection();
    /// let res = posts
    ///     .select((
    ///         title,
    ///         user_id,
    ///         percent_rank().partition_by(user_id).window_order(user_id),
    ///     ))
    ///     .load::<(String, i32, f64)>(connection)?;
    /// let expected = vec![
    ///     ("My first post".to_owned(), 1, 0.0),
    ///     ("About Rust".into(), 1, 0.0),
    ///     ("My first post too".into(), 2, 0.0),
    /// ];
    /// assert_eq!(expected, res);
    /// # Ok(())
    /// # }
    /// ```
    #[window(dialect(
        BuiltInWindowFunctionRequireOrder,
        crate::backend::sql_dialect::built_in_window_function_require_order::NoOrderRequired
    ))]
    #[cfg_attr(
        feature = "mysql_backend",
        window(backends(diesel::mysql::Mysql), require_order = true)
    )]
    fn percent_rank() -> Double;

    /// Cumulative distribution value
    ///
    /// Returns the cumulative distribution,
    /// that is (number of partition rows preceding or peers with current row) / (total partition rows).
    /// The value thus ranges from 1/N to 1.
    ///
    /// This function must be used as window function. You need to call at least one
    /// of the methods [`WindowExpressionMethods`] from to use this function in your `SELECT`
    /// clause. It cannot be used outside of `SELECT` clauses.
    ///
    /// For MySQL this function requires you to call [`.window_order()`](WindowExpressionMethods::window_order())
    ///
    /// ```
    /// # include!("../../doctest_setup.rs");
    /// # use diesel::dsl::*;
    /// #
    /// # fn main() -> QueryResult<()> {
    /// #     use schema::posts::dsl::*;
    /// #     let connection = &mut establish_connection();
    /// let res = posts
    ///     .select((
    ///         title,
    ///         user_id,
    ///         cume_dist().partition_by(user_id).window_order(user_id),
    ///     ))
    ///     .load::<(String, i32, f64)>(connection)?;
    /// let expected = vec![
    ///     ("My first post".to_owned(), 1, 1.0),
    ///     ("About Rust".into(), 1, 1.0),
    ///     ("My first post too".into(), 2, 1.0),
    /// ];
    /// assert_eq!(expected, res);
    /// # Ok(())
    /// # }
    /// ```
    #[window(dialect(
        BuiltInWindowFunctionRequireOrder,
        crate::backend::sql_dialect::built_in_window_function_require_order::NoOrderRequired
    ))]
    #[cfg_attr(
        feature = "mysql_backend",
        window(backends(diesel::mysql::Mysql), require_order = true)
    )]
    fn cume_dist() -> Double;

    /// Bucket number of current row within its partition
    ///
    /// Returns an integer ranging from 1 to the argument value,
    /// dividing the partition as equally as possible.
    ///
    ///
    /// This function must be used as window function. You need to call at least one
    /// of the methods [`WindowExpressionMethods`] from to use this function in your `SELECT`
    /// clause. It cannot be used outside of `SELECT` clauses.
    ///
    /// ```
    /// # include!("../../doctest_setup.rs");
    /// # use diesel::dsl::*;
    /// #
    /// # fn main() -> QueryResult<()> {
    /// #     use schema::posts::dsl::*;
    /// #     let connection = &mut establish_connection();
    /// let res = posts
    ///     .select((title, user_id, ntile(2).partition_by(user_id)))
    ///     .load::<(String, i32, i32)>(connection)?;
    /// let expected = vec![
    ///     ("My first post".to_owned(), 1, 1),
    ///     ("About Rust".into(), 1, 2),
    ///     ("My first post too".into(), 2, 1),
    /// ];
    /// assert_eq!(expected, res);
    /// # Ok(())
    /// # }
    /// ```
    #[window]
    fn ntile(num_buckets: Integer) -> Integer;

    /// Value of argument from row lagging current row within partition
    ///
    /// Returns value evaluated at the row that is one row before the current
    /// row within the partition. If there is no such row, NULL is returned instead.
    ///
    /// See [`lag_with_offset`] and [`lag_with_offset_and_default`] for variants with configurable offset
    /// and default values.
    ///
    /// This function must be used as window function. You need to call at least one
    /// of the methods [`WindowExpressionMethods`] from to use this function in your `SELECT`
    /// clause. It cannot be used outside of `SELECT` clauses.
    ///
    /// For MySQL this function requires you to call [`.window_order()`](WindowExpressionMethods::window_order())
    ///
    /// ```
    /// # include!("../../doctest_setup.rs");
    /// # use diesel::dsl::*;
    /// #
    /// # fn main() -> QueryResult<()> {
    /// #     use schema::posts::dsl::*;
    /// #     let connection = &mut establish_connection();
    /// let res = posts
    ///     .select((
    ///         title,
    ///         user_id,
    ///         lag(id).partition_by(user_id).window_order(user_id),
    ///     ))
    ///     .load::<(String, i32, Option<i32>)>(connection)?;
    /// let expected = vec![
    ///     ("My first post".to_owned(), 1, None),
    ///     ("About Rust".into(), 1, Some(1)),
    ///     ("My first post too".into(), 2, None),
    /// ];
    /// assert_eq!(expected, res);
    /// # Ok(())
    /// # }
    /// ```
    #[window(dialect(
        BuiltInWindowFunctionRequireOrder,
        crate::backend::sql_dialect::built_in_window_function_require_order::NoOrderRequired
    ))]
    #[cfg_attr(
        feature = "mysql_backend",
        window(backends(diesel::mysql::Mysql), require_order = true)
    )]
    fn lag<T: SqlType + SingleValue + IntoNullable<Nullable: SingleValue>>(value: T)
        -> T::Nullable;

    /// Value of argument from row lagging current row within partition
    ///
    /// Returns value evaluated at the row that is offset rows before the current
    /// row within the partition; If there is no such row, NULL is returned instead.
    ///
    /// This function must be used as window function. You need to call at least one
    /// of the methods [`WindowExpressionMethods`] from to use this function in your `SELECT`
    /// clause. It cannot be used outside of `SELECT` clauses.
    ///
    /// For MySQL this function requires you to call [`.window_order()`](WindowExpressionMethods::window_order())
    ///
    /// ```
    /// # include!("../../doctest_setup.rs");
    /// # use diesel::dsl::*;
    /// #
    /// # fn main() -> QueryResult<()> {
    /// #     use schema::posts::dsl::*;
    /// #     let connection = &mut establish_connection();
    /// let res = posts
    ///     .select((
    ///         title,
    ///         user_id,
    ///         lag_with_offset(id, 1)
    ///             .partition_by(user_id)
    ///             .window_order(user_id),
    ///     ))
    ///     .load::<(String, i32, Option<i32>)>(connection)?;
    /// let expected = vec![
    ///     ("My first post".to_owned(), 1, None),
    ///     ("About Rust".into(), 1, Some(1)),
    ///     ("My first post too".into(), 2, None),
    /// ];
    /// assert_eq!(expected, res);
    /// # Ok(())
    /// # }
    /// ```
    #[doc(alias = "lag")]
    #[sql_name = "lag"]
    #[window(dialect(
        BuiltInWindowFunctionRequireOrder,
        crate::backend::sql_dialect::built_in_window_function_require_order::NoOrderRequired
    ))]
    #[cfg_attr(
        feature = "mysql_backend",
        window(backends(diesel::mysql::Mysql), require_order = true)
    )]
    fn lag_with_offset<T: SqlType + SingleValue + IntoNullable<Nullable: SingleValue>>(
        value: T,
        offset: Integer,
    ) -> T::Nullable;

    /// Value of argument from row lagging current row within partition
    ///
    /// Returns value evaluated at the row that is offset rows before the current
    /// row within the partition; if there is no such row, instead returns default
    /// (which must be of a type compatible with value).
    /// Both offset and default are evaluated with respect to the current row.
    /// If omitted, offset defaults to 1 and default to NULL.
    ///
    /// This function returns a nullable value if either the value or the default expression are
    /// nullable.
    ///
    /// This function must be used as window function. You need to call at least one
    /// of the methods [`WindowExpressionMethods`] from to use this function in your `SELECT`
    /// clause. It cannot be used outside of `SELECT` clauses.
    ///
    /// For MySQL this function requires you to call [`.window_order()`](WindowExpressionMethods::window_order())
    ///
    /// ```
    /// # include!("../../doctest_setup.rs");
    /// # use diesel::dsl::*;
    /// #
    /// # #[cfg(not(feature = "mysql"))] // mariadb doesn't support this variant
    /// # fn main() -> QueryResult<()> {
    /// #     use schema::posts::dsl::*;
    /// #     use diesel::sql_types::{Integer, Nullable};
    /// #     let connection = &mut establish_connection();
    /// let res = posts
    ///     .select((
    ///         title,
    ///         user_id,
    ///         lag_with_offset_and_default(id, 1, user_id)
    ///             .partition_by(user_id)
    ///             .window_order(user_id),
    ///     ))
    ///     .load::<(String, i32, i32)>(connection)?;
    /// let expected = vec![
    ///     ("My first post".to_owned(), 1, 1),
    ///     ("About Rust".into(), 1, 1),
    ///     ("My first post too".into(), 2, 2),
    /// ];
    /// assert_eq!(expected, res);
    ///
    /// let res = posts
    ///     .select((
    ///         title,
    ///         user_id,
    ///         lag_with_offset_and_default(None::<i32>.into_sql::<Nullable<Integer>>(), 1, user_id)
    ///             .partition_by(user_id)
    ///             .window_order(user_id),
    ///     ))
    ///     .load::<(String, i32, Option<i32>)>(connection)?;
    /// let expected = vec![
    ///     ("My first post".to_owned(), 1, Some(1)),
    ///     ("About Rust".into(), 1, None),
    ///     ("My first post too".into(), 2, Some(2)),
    /// ];
    /// assert_eq!(expected, res);
    ///
    /// let res = posts
    ///     .select((
    ///         title,
    ///         user_id,
    ///         lag_with_offset_and_default(id, 1, None::<i32>.into_sql::<Nullable<Integer>>())
    ///             .partition_by(user_id)
    ///             .window_order(user_id),
    ///     ))
    ///     .load::<(String, i32, Option<i32>)>(connection)?;
    /// let expected = vec![
    ///     ("My first post".to_owned(), 1, None),
    ///     ("About Rust".into(), 1, Some(1)),
    ///     ("My first post too".into(), 2, None),
    /// ];
    /// assert_eq!(expected, res);
    /// # Ok(())
    /// # }
    /// # #[cfg(feature = "mysql")]
    /// # fn main() {}
    /// ```
    #[doc(alias = "lag")]
    #[sql_name = "lag"]
    #[window(dialect(
        BuiltInWindowFunctionRequireOrder,
        crate::backend::sql_dialect::built_in_window_function_require_order::NoOrderRequired
    ))]
    #[cfg_attr(
        feature = "mysql_backend",
        window(backends(diesel::mysql::Mysql), require_order = true)
    )]
    fn lag_with_offset_and_default<
        T: SqlType
            + SingleValue
            + IntoNotNullable<NotNullable: self::private::SameType<T2::NotNullable>>
            + CombinedNullableValue<T2, T::NotNullable>,
        T2: SqlType + SingleValue + IntoNotNullable,
    >(
        value: T,
        offset: Integer,
        default: T2,
    ) -> T::Out;

    /// Value of argument from row leading current row within partition
    ///
    /// Returns value evaluated at the row that is offset rows after the current
    /// row within the partition; if there is no such row,
    /// `NULL` will be returned instead.
    ///
    /// See [`lead_with_offset`] and [`lead_with_offset_and_default`] for variants with configurable offset
    /// and default values.
    ///
    /// This function must be used as window function. You need to call at least one
    /// of the methods [`WindowExpressionMethods`] from to use this function in your `SELECT`
    /// clause. It cannot be used outside of `SELECT` clauses.
    ///
    /// For MySQL this function requires you to call [`.window_order()`](WindowExpressionMethods::window_order())
    ///
    /// ```
    /// # include!("../../doctest_setup.rs");
    /// # use diesel::dsl::*;
    /// #
    /// # fn main() -> QueryResult<()> {
    /// #     use schema::posts::dsl::*;
    /// #     let connection = &mut establish_connection();
    /// let res = posts
    ///     .select((
    ///         title,
    ///         user_id,
    ///         lead(id).partition_by(user_id).window_order(user_id),
    ///     ))
    ///     .load::<(String, i32, Option<i32>)>(connection)?;
    /// let expected = vec![
    ///     ("My first post".to_owned(), 1, Some(2)),
    ///     ("About Rust".into(), 1, None),
    ///     ("My first post too".into(), 2, None),
    /// ];
    /// assert_eq!(expected, res);
    /// # Ok(())
    /// # }
    /// ```
    #[window(dialect(
        BuiltInWindowFunctionRequireOrder,
        crate::backend::sql_dialect::built_in_window_function_require_order::NoOrderRequired
    ))]
    #[cfg_attr(
        feature = "mysql_backend",
        window(backends(diesel::mysql::Mysql), require_order = true)
    )]
    fn lead<T: SqlType + SingleValue + IntoNullable<Nullable: SingleValue>>(
        value: T,
    ) -> T::Nullable;

    /// Value of argument from row leading current row within partition
    ///
    /// Returns value evaluated at the row that is offset rows after the current
    /// row within the partition; if there is no such row,
    /// `NULL` is returned instead
    ///
    ///
    /// This function must be used as window function. You need to call at least one
    /// of the methods [`WindowExpressionMethods`] from to use this function in your `SELECT`
    /// clause. It cannot be used outside of `SELECT` clauses.
    ///
    /// For MySQL this function requires you to call [`.window_order()`](WindowExpressionMethods::window_order())
    ///
    /// ```
    /// # include!("../../doctest_setup.rs");
    /// # use diesel::dsl::*;
    /// #
    /// # fn main() -> QueryResult<()> {
    /// #     use schema::posts::dsl::*;
    /// #     let connection = &mut establish_connection();
    /// let res = posts
    ///     .select((
    ///         title,
    ///         user_id,
    ///         lead_with_offset(id, 1)
    ///             .partition_by(user_id)
    ///             .window_order(user_id),
    ///     ))
    ///     .load::<(String, i32, Option<i32>)>(connection)?;
    /// let expected = vec![
    ///     ("My first post".to_owned(), 1, Some(2)),
    ///     ("About Rust".into(), 1, None),
    ///     ("My first post too".into(), 2, None),
    /// ];
    /// assert_eq!(expected, res);
    /// # Ok(())
    /// # }
    /// ```
    #[doc(alias = "lead")]
    #[sql_name = "lead"]
    #[window(dialect(
        BuiltInWindowFunctionRequireOrder,
        crate::backend::sql_dialect::built_in_window_function_require_order::NoOrderRequired
    ))]
    #[cfg_attr(
        feature = "mysql_backend",
        window(backends(diesel::mysql::Mysql), require_order = true)
    )]
    fn lead_with_offset<T: SqlType + SingleValue + IntoNullable<Nullable: SingleValue>>(
        value: T,
        offset: Integer,
    ) -> T::Nullable;

    /// Value of argument from row leading current row within partition
    ///
    /// Returns value evaluated at the row that is offset rows after the current
    /// row within the partition; if there is no such row,
    /// instead returns default (which must be of a type compatible with value).
    /// Both offset and default are evaluated with respect to the current row.
    /// If omitted, offset defaults to 1 and default to NULL.
    ///
    /// This function returns a nullable value if either the value or the default expression are
    /// nullable.
    ///
    /// This function must be used as window function. You need to call at least one
    /// of the methods [`WindowExpressionMethods`] from to use this function in your `SELECT`
    /// clause. It cannot be used outside of `SELECT` clauses.
    ///
    /// For MySQL this function requires you to call [`.window_order()`](WindowExpressionMethods::window_order())
    ///
    /// ```
    /// # include!("../../doctest_setup.rs");
    /// # use diesel::dsl::*;
    /// #
    /// # #[cfg(not(feature = "mysql"))] // mariadb doesn't support this variant
    /// # fn main() -> QueryResult<()> {
    /// #     use schema::posts::dsl::*;
    /// #     use diesel::sql_types::{Integer, Nullable};
    /// #     let connection = &mut establish_connection();
    /// let res = posts
    ///     .select((
    ///         title,
    ///         user_id,
    ///         lead_with_offset_and_default(id, 1, user_id)
    ///             .partition_by(user_id)
    ///             .window_order(user_id),
    ///     ))
    ///     .load::<(String, i32, i32)>(connection)?;
    /// let expected = vec![
    ///     ("My first post".to_owned(), 1, 2),
    ///     ("About Rust".into(), 1, 1),
    ///     ("My first post too".into(), 2, 2),
    /// ];
    /// assert_eq!(expected, res);
    ///
    /// let res = posts
    ///     .select((
    ///         title,
    ///         user_id,
    ///         lead_with_offset_and_default(None::<i32>.into_sql::<Nullable<Integer>>(), 1, user_id)
    ///             .partition_by(user_id)
    ///             .window_order(user_id),
    ///     ))
    ///     .load::<(String, i32, Option<i32>)>(connection)?;
    /// let expected = vec![
    ///     ("My first post".to_owned(), 1, None),
    ///     ("About Rust".into(), 1, Some(1)),
    ///     ("My first post too".into(), 2, Some(2)),
    /// ];
    /// assert_eq!(expected, res);
    ///
    /// let res = posts
    ///     .select((
    ///         title,
    ///         user_id,
    ///         lead_with_offset_and_default(id, 1, None::<i32>.into_sql::<Nullable<Integer>>())
    ///             .partition_by(user_id)
    ///             .window_order(user_id),
    ///     ))
    ///     .load::<(String, i32, Option<i32>)>(connection)?;
    /// let expected = vec![
    ///     ("My first post".to_owned(), 1, Some(2)),
    ///     ("About Rust".into(), 1, None),
    ///     ("My first post too".into(), 2, None),
    /// ];
    /// assert_eq!(expected, res);
    /// # Ok(())
    /// # }
    /// # #[cfg(feature = "mysql")]
    /// fn main() {}
    /// ```
    #[doc(alias = "lead")]
    #[sql_name = "lead"]
    #[window(dialect(
        BuiltInWindowFunctionRequireOrder,
        crate::backend::sql_dialect::built_in_window_function_require_order::NoOrderRequired
    ))]
    #[cfg_attr(
        feature = "mysql_backend",
        window(backends(diesel::mysql::Mysql), require_order = true)
    )]
    fn lead_with_offset_and_default<
        T: SqlType
            + SingleValue
            + IntoNotNullable<NotNullable: self::private::SameType<T2::NotNullable>>
            + CombinedNullableValue<T2, T::NotNullable>,
        T2: SqlType + SingleValue + IntoNotNullable,
    >(
        value: T,
        offset: Integer,
        default: T2,
    ) -> T::Out;

    /// Value of argument from first row of window frame
    ///
    /// Returns value evaluated at the row that is the first row of the window frame.
    ///
    ///
    /// This function must be used as window function. You need to call at least one
    /// of the methods [`WindowExpressionMethods`] from to use this function in your `SELECT`
    /// clause. It cannot be used outside of `SELECT` clauses.
    ///
    /// ```
    /// # include!("../../doctest_setup.rs");
    /// # use diesel::dsl::*;
    /// #
    /// # fn main() -> QueryResult<()> {
    /// #     use schema::posts::dsl::*;
    /// #     let connection = &mut establish_connection();
    /// let res = posts
    ///     .select((title, user_id, first_value(id).partition_by(user_id)))
    ///     .load::<(String, i32, i32)>(connection)?;
    /// let expected = vec![
    ///     ("My first post".to_owned(), 1, 1),
    ///     ("About Rust".into(), 1, 1),
    ///     ("My first post too".into(), 2, 3),
    /// ];
    /// assert_eq!(expected, res);
    /// # Ok(())
    /// # }
    /// ```
    #[window]
    fn first_value<T: SqlType + SingleValue>(value: T) -> T;

    /// Value of argument from last row of window frame
    ///
    /// Returns value evaluated at the row that is the last row of the window frame.
    ///
    ///
    /// This function must be used as window function. You need to call at least one
    /// of the methods [`WindowExpressionMethods`] from to use this function in your `SELECT`
    /// clause. It cannot be used outside of `SELECT` clauses.
    ///
    /// ```
    /// # include!("../../doctest_setup.rs");
    /// # use diesel::dsl::*;
    /// #
    /// # fn main() -> QueryResult<()> {
    /// #     use schema::posts::dsl::*;
    /// #     let connection = &mut establish_connection();
    /// let res = posts
    ///     .select((title, user_id, last_value(id).partition_by(user_id)))
    ///     .load::<(String, i32, i32)>(connection)?;
    /// let expected = vec![
    ///     ("My first post".to_owned(), 1, 2),
    ///     ("About Rust".into(), 1, 2),
    ///     ("My first post too".into(), 2, 3),
    /// ];
    /// assert_eq!(expected, res);
    /// # Ok(())
    /// # }
    /// ```
    #[window]
    fn last_value<T: SqlType + SingleValue>(value: T) -> T;

    /// Value of argument from N-th row of window frame
    ///
    /// Returns value evaluated at the row that is the n'th row of the window frame (counting from 1);
    /// returns NULL if there is no such row.
    ///
    ///
    /// This function must be used as window function. You need to call at least one
    /// of the methods [`WindowExpressionMethods`] from to use this function in your `SELECT`
    /// clause. It cannot be used outside of `SELECT` clauses.
    ///
    /// ```
    /// # include!("../../doctest_setup.rs");
    /// # use diesel::dsl::*;
    /// #
    /// # fn main() -> QueryResult<()> {
    /// #     use schema::posts::dsl::*;
    /// #     let connection = &mut establish_connection();
    /// let res = posts
    ///     .select((title, user_id, nth_value(id, 2).partition_by(user_id)))
    ///     .load::<(String, i32, Option<i32>)>(connection)?;
    /// let expected = vec![
    ///     ("My first post".to_owned(), 1, Some(2)),
    ///     ("About Rust".into(), 1, Some(2)),
    ///     ("My first post too".into(), 2, None),
    /// ];
    /// assert_eq!(expected, res);
    /// # Ok(())
    /// # }
    /// ```
    #[window]
    fn nth_value<T: SqlType + SingleValue + IntoNullable<Nullable: SingleValue>>(
        value: T,
        n: Integer,
    ) -> T::Nullable;
}

mod private {
    pub trait SameType<T> {}

    impl<T> SameType<T> for T {}
}
