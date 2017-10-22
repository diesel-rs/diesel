use query_source::Table;

/// Sets the limit clause of a query. If there was already a limit clause, it
/// will be overridden. This is automatically implemented for the various query
/// builder types.
pub trait LimitDsl {
    type Output;

    /// Creates a `SQL LIMIT` statement.
    /// Limits the number of records returned by the integer passed in.
    ///
    /// # Example
    ///
    /// ```rust
    /// # #[macro_use] extern crate diesel;
    /// # include!("../doctest_setup.rs");
    /// # use schema::users;
    /// #
    /// # fn main() {
    /// #   use users::dsl::*;
    /// #   let connection = establish_connection();
    /// #   diesel::delete(users).execute(&connection).unwrap();
    /// #
    /// # let new_users = vec![
    /// #    NewUser { name: "Sean".to_string(), },
    /// #    NewUser { name: "Bastien".to_string(), },
    /// #    NewUser { name: "Pascal".to_string(), },
    /// # ];
    /// #
    /// # diesel::insert_into(users)
    /// #    .values(&new_users)
    /// #    .execute(&connection)
    /// #    .unwrap();
    /// #
    /// // Using a limit
    /// let limited = users.select(name)
    ///     .order(id)
    ///     .limit(1)
    ///     .load::<String>(&connection)
    ///     .unwrap();
    ///
    /// // Without a limit
    /// let no_limit = users.select(name)
    ///     .order(id)
    ///     .load::<String>(&connection)
    ///     .unwrap();
    ///
    /// assert_eq!(vec!["Sean".to_string()], limited);
    /// assert_eq!(vec!["Sean".to_string(), "Bastien".to_string(), "Pascal".to_string()], no_limit);
    /// # }
    /// ```
    fn limit(self, limit: i64) -> Self::Output;
}

impl<T> LimitDsl for T
where
    T: Table,
    T::Query: LimitDsl,
{
    type Output = <T::Query as LimitDsl>::Output;

    fn limit(self, limit: i64) -> Self::Output {
        self.as_query().limit(limit)
    }
}
