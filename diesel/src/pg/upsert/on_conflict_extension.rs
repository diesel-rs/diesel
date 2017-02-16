pub use super::on_conflict_clause::*;

/// Adds extension methods related to PG upsert
pub trait OnConflictExtension {
    /// Adds `ON CONFLICT DO NOTHING` to the insert statement, without
    /// specifying any columns or constraints to restrict the conflict to.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # #[macro_use] extern crate diesel;
    /// # #[macro_use] extern crate diesel_codegen;
    /// # include!("src/doctest_setup.rs");
    /// #
    /// # table! {
    /// #     users {
    /// #         id -> Integer,
    /// #         name -> VarChar,
    /// #     }
    /// # }
    /// #
    /// # #[derive(Clone, Copy, Insertable)]
    /// # #[table_name="users"]
    /// # struct User<'a> {
    /// #     id: i32,
    /// #     name: &'a str,
    /// # }
    /// #
    /// # fn main() {
    /// #     use self::users::dsl::*;
    /// use self::diesel::pg::upsert::*;
    ///
    /// #     let conn = establish_connection();
    /// #     conn.execute("TRUNCATE TABLE users").unwrap();
    /// let user = User { id: 1, name: "Sean", };
    ///
    /// let inserted_row_count = diesel::insert(&user.on_conflict_do_nothing())
    ///     .into(users).execute(&conn);
    /// assert_eq!(Ok(1), inserted_row_count);
    ///
    /// let inserted_row_count = diesel::insert(&user.on_conflict_do_nothing())
    ///     .into(users).execute(&conn);
    /// assert_eq!(Ok(0), inserted_row_count);
    /// # }
    /// ```
    ///
    /// ```rust
    /// # #[macro_use] extern crate diesel;
    /// # #[macro_use] extern crate diesel_codegen;
    /// # include!("src/doctest_setup.rs");
    /// #
    /// # table! {
    /// #     users {
    /// #         id -> Integer,
    /// #         name -> VarChar,
    /// #     }
    /// # }
    /// #
    /// # #[derive(Clone, Copy, Insertable)]
    /// # #[table_name="users"]
    /// # struct User<'a> {
    /// #     id: i32,
    /// #     name: &'a str,
    /// # }
    /// #
    /// # fn main() {
    /// #     use self::users::dsl::*;
    /// use self::diesel::pg::upsert::*;
    ///
    /// #     let conn = establish_connection();
    /// #     conn.execute("TRUNCATE TABLE users").unwrap();
    /// let user = User { id: 1, name: "Sean", };
    ///
    /// let inserted_row_count = diesel::insert(&vec![user, user].on_conflict_do_nothing())
    ///     .into(users).execute(&conn);
    /// assert_eq!(Ok(1), inserted_row_count);
    /// # }
    /// ```
    fn on_conflict_do_nothing(&self) -> OnConflictDoNothing<&Self> {
        OnConflictDoNothing::new(self)
    }
}

impl<T> OnConflictExtension for T {
}
