use super::delete_statement::DeleteStatement;
use super::insert_statement::{Insert, InsertOrIgnore, Replace};
use super::{
    IncompleteInsertStatement, IntoUpdateTarget, SelectStatement, SqlQuery, UpdateStatement,
};
use dsl::Select;
use expression::Expression;
use query_dsl::methods::SelectDsl;

/// Creates an `UPDATE` statement.
///
/// When a table is passed to `update`, every row in the table will be updated.
/// You can narrow this scope by calling [`filter`] on the table before passing it in,
/// which will result in `UPDATE your_table SET ... WHERE args_to_filter`.
///
/// Passing a type which implements `Identifiable` is the same as passing
/// `some_table.find(some_struct.id())`.
///
/// [`filter`]: query_builder/struct.UpdateStatement.html#method.filter
///
/// # Examples
///
/// ```rust
/// # #[macro_use] extern crate diesel;
/// # include!("../doctest_setup.rs");
/// #
/// # #[cfg(feature = "postgres")]
/// # fn main() {
/// #     use schema::users::dsl::*;
/// #     let connection = establish_connection();
/// let updated_row = diesel::update(users.filter(id.eq(1)))
///     .set(name.eq("James"))
///     .get_result(&connection);
/// // On backends that support it, you can call `get_result` instead of `execute`
/// // to have `RETURNING *` automatically appended to the query. Alternatively, you
/// // can explicitly return an expression by using the `returning` method before
/// // getting the result.
/// assert_eq!(Ok((1, "James".to_string())), updated_row);
/// # }
/// # #[cfg(not(feature = "postgres"))]
/// # fn main() {}
/// ```
///
/// To update multiple columns, give [`set`] a tuple argument:
///
/// [`set`]: query_builder/struct.UpdateStatement.html#method.set
///
/// ```rust
/// # #[macro_use] extern crate diesel;
/// # include!("../doctest_setup.rs");
/// #
/// # table! {
/// #     users {
/// #         id -> Integer,
/// #         name -> VarChar,
/// #         surname -> VarChar,
/// #     }
/// # }
/// #
/// # #[cfg(feature = "postgres")]
/// # fn main() {
/// # use users::dsl::*;
/// # let connection = establish_connection();
/// # connection.execute("DROP TABLE users").unwrap();
/// # connection.execute("CREATE TABLE users (
/// #     id SERIAL PRIMARY KEY,
/// #     name VARCHAR,
/// #     surname VARCHAR)").unwrap();
/// # connection.execute("INSERT INTO users(name, surname) VALUES('Sean', 'Griffin')").unwrap();
///
/// let updated_row = diesel::update(users.filter(id.eq(1)))
///     .set((name.eq("James"), surname.eq("Bond")))
///     .get_result(&connection);
///
/// assert_eq!(Ok((1, "James".to_string(), "Bond".to_string())), updated_row);
/// # }
/// # #[cfg(not(feature = "postgres"))]
/// # fn main() {}
/// ```
pub fn update<T: IntoUpdateTarget>(source: T) -> UpdateStatement<T::Table, T::WhereClause> {
    UpdateStatement::new(source.into_update_target())
}

/// Creates a `DELETE` statement.
///
/// When a table is passed to `delete`,
/// every row in the table will be deleted.
/// This scope can be narrowed by calling [`filter`]
/// on the table before it is passed in.
///
/// [`filter`]: query_builder/struct.DeleteStatement.html#method.filter
///
/// # Examples
///
/// ### Deleting a single record:
///
/// ```rust
/// # #[macro_use] extern crate diesel;
/// # include!("../doctest_setup.rs");
/// #
/// # fn main() {
/// #     delete();
/// # }
/// #
/// # fn delete() -> QueryResult<()> {
/// #     use schema::users::dsl::*;
/// #     let connection = establish_connection();
/// #     let get_count = || users.count().first::<i64>(&connection);
/// let old_count = get_count();
/// diesel::delete(users.filter(id.eq(1))).execute(&connection)?;
/// assert_eq!(old_count.map(|count| count - 1), get_count());
/// # Ok(())
/// # }
/// ```
///
/// ### Deleting a whole table:
///
/// ```rust
/// # #[macro_use] extern crate diesel;
/// # include!("../doctest_setup.rs");
/// #
/// # fn main() {
/// #     delete();
/// # }
/// #
/// # fn delete() -> QueryResult<()> {
/// #     use schema::users::dsl::*;
/// #     let connection = establish_connection();
/// #     let get_count = || users.count().first::<i64>(&connection);
/// diesel::delete(users).execute(&connection)?;
/// assert_eq!(Ok(0), get_count());
/// # Ok(())
/// # }
/// ```
pub fn delete<T: IntoUpdateTarget>(source: T) -> DeleteStatement<T::Table, T::WhereClause> {
    let target = source.into_update_target();
    DeleteStatement::new(target.table, target.where_clause)
}

/// Creates an `INSERT` statement for the target table.
///
/// You may add data by calling [`values()`] or [`default_values()`]
/// as shown in the examples.
///
/// [`values()`]: query_builder/struct.IncompleteInsertStatement.html#method.values
/// [`default_values()`]: query_builder/struct.IncompleteInsertStatement.html#method.default_values
///
/// Backends that support the `RETURNING` clause, such as PostgreSQL,
/// can return the inserted rows by calling [`.get_results`] instead of [`.execute`].
///
/// [`.get_results`]: query_dsl/trait.RunQueryDsl.html#method.get_results
/// [`.execute`]: query_dsl/trait.RunQueryDsl.html#tymethod.execute
///
/// # Examples
///
/// ```rust
/// # #[macro_use] extern crate diesel;
/// # include!("../doctest_setup.rs");
/// #
/// # fn main() {
/// #     use schema::users::dsl::*;
/// #     let connection = establish_connection();
/// let rows_inserted = diesel::insert_into(users)
///     .values(&name.eq("Sean"))
///     .execute(&connection);
///
/// assert_eq!(Ok(1), rows_inserted);
///
/// let new_users = vec![
///     name.eq("Tess"),
///     name.eq("Jim"),
/// ];
///
/// let rows_inserted = diesel::insert_into(users)
///     .values(&new_users)
///     .execute(&connection);
///
/// assert_eq!(Ok(2), rows_inserted);
/// # }
/// ```
///
/// ### Using a tuple for values
///
/// ```rust
/// # #[macro_use] extern crate diesel;
/// # include!("../doctest_setup.rs");
/// #
/// # fn main() {
/// #     use schema::users::dsl::*;
/// #     let connection = establish_connection();
/// #     diesel::delete(users).execute(&connection).unwrap();
/// let new_user = (id.eq(1), name.eq("Sean"));
/// let rows_inserted = diesel::insert_into(users)
///     .values(&new_user)
///     .execute(&connection);
///
/// assert_eq!(Ok(1), rows_inserted);
///
/// let new_users = vec![
///     (id.eq(2), name.eq("Tess")),
///     (id.eq(3), name.eq("Jim")),
/// ];
///
/// let rows_inserted = diesel::insert_into(users)
///     .values(&new_users)
///     .execute(&connection);
///
/// assert_eq!(Ok(2), rows_inserted);
/// # }
/// ```
///
/// ### Using struct for values
///
/// ```rust
/// # #[macro_use] extern crate diesel;
/// # include!("../doctest_setup.rs");
/// # use schema::users;
/// #
/// #[derive(Insertable)]
/// #[table_name = "users"]
/// struct NewUser<'a> {
///     name: &'a str,
/// }
///
/// # fn main() {
/// #     use schema::users::dsl::*;
/// #     let connection = establish_connection();
/// // Insert one record at a time
///
/// let new_user = NewUser { name: "Ruby Rhod" };
///
/// diesel::insert_into(users)
///     .values(&new_user)
///     .execute(&connection)
///     .unwrap();
///
/// // Insert many records
///
/// let new_users = vec![
///     NewUser { name: "Leeloo Multipass", },
///     NewUser { name: "Korben Dallas", },
/// ];
///
/// let inserted_names = diesel::insert_into(users)
///     .values(&new_users)
///     .execute(&connection)
///     .unwrap();
/// # }
/// ```
///
/// ### Insert from select
///
/// When inserting from a select statement,
/// the column list can be specified with [`.into_columns`].
/// (See also [`SelectStatement::insert_into`], which generally
/// reads better for select statements)
///
/// [`SelectStatement::insert_into`]: prelude/trait.Insertable.html#method.insert_into
/// [`.into_columns`]: query_builder/struct.InsertStatement.html#method.into_columns
///
/// ```rust
/// # #[macro_use] extern crate diesel;
/// # include!("../doctest_setup.rs");
/// #
/// # fn main() {
/// #     run_test().unwrap();
/// # }
/// #
/// # fn run_test() -> QueryResult<()> {
/// #     use schema::{posts, users};
/// #     let conn = establish_connection();
/// #     diesel::delete(posts::table).execute(&conn)?;
/// let new_posts = users::table
///     .select((
///         users::name.concat("'s First Post"),
///         users::id,
///     ));
/// diesel::insert_into(posts::table)
///     .values(new_posts)
///     .into_columns((posts::title, posts::user_id))
///     .execute(&conn)?;
///
/// let inserted_posts = posts::table
///     .select(posts::title)
///     .load::<String>(&conn)?;
/// let expected = vec!["Sean's First Post", "Tess's First Post"];
/// assert_eq!(expected, inserted_posts);
/// #     Ok(())
/// # }
/// ```
///
/// ### With return value
///
/// ```rust
/// # #[macro_use] extern crate diesel;
/// # include!("../doctest_setup.rs");
/// #
/// # #[cfg(feature = "postgres")]
/// # fn main() {
/// #     use schema::users::dsl::*;
/// #     let connection = establish_connection();
/// let inserted_names = diesel::insert_into(users)
///     .values(&vec![
///         name.eq("Diva Plavalaguna"),
///         name.eq("Father Vito Cornelius"),
///     ])
///     .returning(name)
///     .get_results(&connection);
/// assert_eq!(Ok(vec!["Diva Plavalaguna".to_string(), "Father Vito Cornelius".to_string()]), inserted_names);
/// # }
/// # #[cfg(not(feature = "postgres"))]
/// # fn main() {}
/// ```
pub fn insert_into<T>(target: T) -> IncompleteInsertStatement<T, Insert> {
    IncompleteInsertStatement::new(target, Insert)
}

/// Creates an `INSERT [OR] IGNORE` statement.
///
/// If a constraint violation fails, the database will ignore the offending
/// row and continue processing any subsequent rows. This function is only
/// available with MySQL and SQLite.
///
/// With PostgreSQL, similar functionality is provided by [`on_conflict_do_nothing`].
///
/// [`on_conflict_do_nothing`]: query_builder/insert_statement/struct.InsertStatement.html#method.on_conflict_do_nothing
///
/// # Example
///
/// ```rust
/// # #[macro_use] extern crate diesel;
/// # include!("../doctest_setup.rs");
/// #
/// # fn main() {
/// #     run_test().unwrap();
/// # }
/// #
/// # #[cfg(not(feature = "postgres"))]
/// # fn run_test() -> QueryResult<()> {
/// #     use schema::users::dsl::*;
/// #     use diesel::{delete, insert_or_ignore_into};
/// #
/// #     let connection = establish_connection();
/// #     diesel::delete(users).execute(&connection)?;
/// insert_or_ignore_into(users)
///     .values((id.eq(1), name.eq("Jim")))
///     .execute(&connection)?;
///
/// insert_or_ignore_into(users)
///     .values(&vec![
///         (id.eq(1), name.eq("Sean")),
///         (id.eq(2), name.eq("Tess")),
///     ])
///     .execute(&connection)?;
///
/// let names = users.select(name).order(id).load::<String>(&connection)?;
/// assert_eq!(vec![String::from("Jim"), String::from("Tess")], names);
/// #     Ok(())
/// # }
/// #
/// # #[cfg(feature = "postgres")]
/// # fn run_test() -> QueryResult<()> {
/// #     Ok(())
/// # }
/// ```
pub fn insert_or_ignore_into<T>(target: T) -> IncompleteInsertStatement<T, InsertOrIgnore> {
    IncompleteInsertStatement::new(target, InsertOrIgnore)
}

/// Creates a bare select statement, with no from clause. Primarily used for
/// testing diesel itself, but likely useful for third party crates as well. The
/// given expressions must be selectable from anywhere.
pub fn select<T>(expression: T) -> Select<SelectStatement<()>, T>
where
    T: Expression,
    SelectStatement<()>: SelectDsl<T>,
{
    SelectStatement::simple(()).select(expression)
}

/// Creates a `REPLACE` statement.
///
/// If a constraint violation fails, the database will attempt to replace the
/// offending row instead. This function is only available with MySQL and
/// SQLite.
///
/// # Example
///
/// ```rust
/// # #[macro_use] extern crate diesel;
/// # include!("../doctest_setup.rs");
/// #
/// # #[cfg(not(feature = "postgres"))]
/// # fn main() {
/// #     use schema::users::dsl::*;
/// #     use diesel::{insert_into, replace_into};
/// #
/// #     let conn = establish_connection();
/// #     conn.execute("DELETE FROM users").unwrap();
/// replace_into(users)
///     .values(&vec![
///         (id.eq(1), name.eq("Sean")),
///         (id.eq(2), name.eq("Tess")),
///     ])
///     .execute(&conn)
///     .unwrap();
///
/// replace_into(users)
///     .values((id.eq(1), name.eq("Jim")))
///     .execute(&conn)
///     .unwrap();
///
/// let names = users.select(name).order(id).load::<String>(&conn);
/// assert_eq!(Ok(vec!["Jim".into(), "Tess".into()]), names);
/// # }
/// # #[cfg(feature = "postgres")] fn main() {}
pub fn replace_into<T>(target: T) -> IncompleteInsertStatement<T, Replace> {
    IncompleteInsertStatement::new(target, Replace)
}

/// Construct a full SQL query using raw SQL.
///
/// This function exists for cases where a query needs to be written that is not
/// supported by the query builder. Unlike most queries in Diesel, `sql_query`
/// will deserialize its data by name, not by index. That means that you cannot
/// deserialize into a tuple, and structs which you deserialize from this
/// function will need to have `#[derive(QueryableByName)]`
///
/// # Safety
///
/// The implementation of `QueryableByName` will assume that columns with a
/// given name will have a certain type. The compiler will be unable to verify
/// that the given type is correct. If your query returns a column of an
/// unexpected type, the result may have the wrong value, or return an error.
///
/// # Example
///
/// ```rust
/// # #[macro_use] extern crate diesel;
/// # include!("../doctest_setup.rs");
/// #
/// # use schema::users;
/// #
/// # #[derive(QueryableByName, Debug, PartialEq)]
/// # #[table_name="users"]
/// # struct User {
/// #     id: i32,
/// #     name: String,
/// # }
/// #
/// # fn main() {
/// #     use diesel::sql_query;
/// #
/// #     let connection = establish_connection();
/// let users = sql_query("SELECT * FROM users ORDER BY id")
///     .load(&connection);
/// let expected_users = vec![
///     User { id: 1, name: "Sean".into() },
///     User { id: 2, name: "Tess".into() },
/// ];
/// assert_eq!(Ok(expected_users), users);
/// # }
/// ```
pub fn sql_query<T: Into<String>>(query: T) -> SqlQuery<()> {
    SqlQuery::new((), query.into())
}
