use query_source::Table;

/// Sets the limit clause of a query. If there was already a limit clause, it
/// will be overridden. This is automatically implemented for the various query
/// builder types.
pub trait LimitDsl {
    type Output;

    fn limit(self, limit: i64) -> Self::Output;
}

impl<T> LimitDsl for T
where
    T: Table,
    T::Query: LimitDsl,
{
    type Output = <T::Query as LimitDsl>::Output;

    /// Set the `LIMIT` on the query.
    ///
    /// # Example
    ///
    /// ```rust
    /// # #[macro_use] extern crate diesel_codegen;
    /// # #[macro_use] extern crate diesel;
    /// # use diesel::insert_into;
    /// # include!("../doctest_setup.rs");
    /// # table! {
    /// #     users {
    /// #         id -> Integer,
    /// #         name -> VarChar,
    /// #     }
    /// # }
    /// #
    /// # #[derive(Queryable, PartialEq, Debug)]
    /// # struct User {
    /// #     id: i32,
    /// #     name: String,
    /// # }
    /// #
    /// # fn main() {
    /// #     let connection = establish_connection();
    /// insert_into(users::table)
    /// .values(&vec![
    ///     users::name.eq("Sean"),
    ///     users::name.eq("Tess"),
    ///     users::name.eq("Pascal"),
    /// ])
    /// .execute(&connection);
    ///
    /// let users = users::table.order(users::id.asc()).limit(2).load::<User>(&connection).unwrap();
    /// assert_eq!(users, vec![User { id: 1, name: "Sean".into() }, User { id: 2, name: "Tess".into() }]);
    /// # }
    /// ```
    fn limit(self, limit: i64) -> Self::Output {
        self.as_query().limit(limit)
    }
}
