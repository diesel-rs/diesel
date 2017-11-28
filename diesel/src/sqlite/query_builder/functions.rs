#![cfg(feature = "with-deprecated")]
use query_builder::insert_statement::{DeprecatedIncompleteInsertStatement, Replace};

/// Creates a SQLite `INSERT OR REPLACE` statement. If a constraint violation
/// fails, SQLite will attempt to replace the offending row instead.
///
/// # Example
///
/// ```rust
/// # #[macro_use] extern crate diesel;
/// # include!("../../doctest_setup.rs");
/// #
/// # table! {
/// #     users {
/// #         id -> Integer,
/// #         name -> VarChar,
/// #     }
/// # }
/// #
/// # #[derive(Insertable)]
/// # #[table_name="users"]
/// # struct User<'a> {
/// #     id: i32,
/// #     name: &'a str,
/// # }
/// #
/// #
/// # fn main() {
/// #     use users::dsl::*;
/// #     use diesel::{insert_into, insert_or_replace};
/// #     use diesel::sqlite::SqliteConnection;
/// #
/// #     let conn = SqliteConnection::establish(":memory:").unwrap();
/// #     conn.execute("CREATE TABLE users (id INTEGER PRIMARY KEY AUTOINCREMENT, name VARCHAR)").unwrap();
/// insert_into(users).values(&NewUser::new("Sean")).execute(&conn).unwrap();
/// insert_into(users).values(&NewUser::new("Tess")).execute(&conn).unwrap();
///
/// let new_user = User { id: 1, name: "Jim" };
/// insert_or_replace(&new_user).into(users).execute(&conn).unwrap();
///
/// let names = users.select(name).order(id).load::<String>(&conn);
/// assert_eq!(Ok(vec!["Jim".into(), "Tess".into()]), names);
/// # }
/// ```
#[deprecated(since = "0.99.0", note = "use `replace_into` instead")]
pub fn insert_or_replace<T: ?Sized>(
    records: &T,
) -> DeprecatedIncompleteInsertStatement<&T, Replace> {
    DeprecatedIncompleteInsertStatement::new(records, Replace)
}
