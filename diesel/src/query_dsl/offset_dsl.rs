use query_source::Table;

/// Sets the offset clause of a query. If there was already a offset clause, it
/// will be overridden. This is automatically implemented for the various query
/// builder types.
pub trait OffsetDsl {
    type Output;

    /// Creates a `SQL OFFSET` statement.
    /// Offsets the number of records returned by the integer passed in.
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
    /// // Using an offset
    /// let offset = users.select(name)
    ///     .order(id)
    ///     .limit(2)
    ///     .offset(1)
    ///     .load::<String>(&connection)
    ///     .unwrap();
    ///
    /// // No Offset
    /// let no_offset = users.select(name)
    ///     .order(id)
    ///     .limit(2)
    ///     .load::<String>(&connection)
    ///     .unwrap();
    ///
    /// assert_eq!(vec!["Bastien".to_string(), "Pascal".to_string()], offset);
    /// assert_eq!(vec!["Sean".to_string(), "Bastien".to_string()], no_offset);
    /// # }
    /// ```
    fn offset(self, offset: i64) -> Self::Output;
}

impl<T> OffsetDsl for T
where
    T: Table,
    T::Query: OffsetDsl,
{
    type Output = <T::Query as OffsetDsl>::Output;

    fn offset(self, offset: i64) -> Self::Output {
        self.as_query().offset(offset)
    }
}
