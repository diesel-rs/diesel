pub use super::on_conflict_clause::*;
pub use super::on_conflict_target::*;

/// Adds extension methods related to PG upsert
pub trait OnConflictExtension {
    /// Adds `ON CONFLICT DO NOTHING` to the insert statement, without
    /// specifying any columns or constraints to restrict the conflict to.
    ///
    /// # Examples
    ///
    /// ### Single Record
    ///
    /// ```rust
    /// # #[macro_use] extern crate diesel;
    /// # #[macro_use] extern crate diesel_codegen;
    /// # include!("on_conflict_docs_setup.rs");
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
    /// ### Vec of Records
    ///
    /// ```rust
    /// # #[macro_use] extern crate diesel;
    /// # #[macro_use] extern crate diesel_codegen;
    /// # include!("on_conflict_docs_setup.rs");
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
    ///
    /// ### Slice of records
    ///
    /// ```rust
    /// # #[macro_use] extern crate diesel;
    /// # #[macro_use] extern crate diesel_codegen;
    /// # include!("on_conflict_docs_setup.rs");
    /// #
    /// # fn main() {
    /// #     use self::users::dsl::*;
    /// use self::diesel::pg::upsert::*;
    ///
    /// #     let conn = establish_connection();
    /// #     conn.execute("TRUNCATE TABLE users").unwrap();
    /// let user = User { id: 1, name: "Sean", };
    ///
    /// let new_users: &[User] = &[user, user];
    /// let inserted_row_count = diesel::insert(&new_users.on_conflict_do_nothing())
    ///     .into(users).execute(&conn);
    /// assert_eq!(Ok(1), inserted_row_count);
    /// # }
    /// ```
    fn on_conflict_do_nothing(&self) -> OnConflictDoNothing<&Self> {
        OnConflictDoNothing::new(self)
    }

    /// Adds an `ON CONFLICT` to the insert statement, performing the action
    /// specified by `Action` if a conflict occurs for `Target`.
    ///
    /// `Target` can be one of:
    ///
    /// - A column
    /// - A tuple of columns
    /// - [`on_constraint("constraint_name")`](fn.on_constraint.html)
    ///
    /// `Action` can be one of:
    ///
    /// - [`do_nothing()`](fn.do_nothing.html)
    /// - [`do_update()`](fn.do_update.html)
    ///
    /// # Examples
    ///
    /// ### Specifying a column as the target
    ///
    /// ```rust
    /// # #[macro_use] extern crate diesel;
    /// # #[macro_use] extern crate diesel_codegen;
    /// # include!("on_conflict_docs_setup.rs");
    /// #
    /// # fn main() {
    /// #     use self::users::dsl::*;
    /// use self::diesel::pg::upsert::*;
    ///
    /// #     let conn = establish_connection();
    /// #     conn.execute("TRUNCATE TABLE users").unwrap();
    /// conn.execute("CREATE UNIQUE INDEX users_name ON users (name)").unwrap();
    /// let user = User { id: 1, name: "Sean", };
    /// let same_name_different_id = User { id: 2, name: "Sean" };
    /// let same_id_different_name = User { id: 1, name: "Pascal" };
    ///
    /// assert_eq!(Ok(1), diesel::insert(&user).into(users).execute(&conn));
    ///
    /// let inserted_row_count = diesel::insert(
    ///     &same_name_different_id.on_conflict(name, do_nothing())
    /// ).into(users).execute(&conn);
    /// assert_eq!(Ok(0), inserted_row_count);
    ///
    /// let pk_conflict_result = diesel::insert(
    ///     &same_id_different_name.on_conflict(name, do_nothing())
    /// ).into(users).execute(&conn);
    /// assert!(pk_conflict_result.is_err());
    /// # }
    /// ```
    ///
    /// ### Specifying multiple columns as the target
    ///
    /// ```rust
    /// # #[macro_use] extern crate diesel;
    /// # #[macro_use] extern crate diesel_codegen;
    /// # include!("../../doctest_setup.rs");
    /// #
    /// # table! {
    /// #     users {
    /// #         id -> Integer,
    /// #         name -> VarChar,
    /// #         hair_color -> VarChar,
    /// #     }
    /// # }
    /// #
    /// # #[derive(Clone, Copy, Insertable)]
    /// # #[table_name="users"]
    /// # struct User<'a> {
    /// #     id: i32,
    /// #     name: &'a str,
    /// #     hair_color: &'a str,
    /// # }
    /// #
    /// # fn main() {
    /// #     use self::users::dsl::*;
    /// use self::diesel::pg::upsert::*;
    ///
    /// #     let conn = establish_connection();
    /// #     conn.execute("DROP TABLE users").unwrap();
    /// #     conn.execute("CREATE TABLE users (id SERIAL PRIMARY KEY, name TEXT, hair_color TEXT)").unwrap();
    /// conn.execute("CREATE UNIQUE INDEX users_name_hair_color ON users (name, hair_color)").unwrap();
    /// let user = User { id: 1, name: "Sean", hair_color: "black" };
    /// let same_name_different_hair_color = User { id: 2, name: "Sean", hair_color: "brown" };
    /// let same_same_name_same_hair_color = User { id: 3, name: "Sean", hair_color: "black" };
    ///
    /// assert_eq!(Ok(1), diesel::insert(&user).into(users).execute(&conn));
    ///
    /// let inserted_row_count = diesel::insert(
    ///     &same_name_different_hair_color.on_conflict((name, hair_color), do_nothing())
    /// ).into(users).execute(&conn);
    /// assert_eq!(Ok(1), inserted_row_count);
    ///
    /// let inserted_row_count = diesel::insert(
    ///     &same_same_name_same_hair_color.on_conflict((name, hair_color), do_nothing())
    /// ).into(users).execute(&conn);
    /// assert_eq!(Ok(0), inserted_row_count);
    /// # }
    /// ```
    ///
    /// See the documentation for [`on_constraint`](fn.on_constraint.html) and [`do_update`] for
    /// more examples.
    fn on_conflict<Target, Action>(
        &self,
        target: Target,
        action: Action,
    ) -> OnConflict<&Self, ConflictTarget<Target>, Action> {
        OnConflict::new(self, ConflictTarget(target), action)
    }
}

impl<T: ?Sized> OnConflictExtension for T {}
