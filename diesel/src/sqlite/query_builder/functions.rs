use expression::predicates::Or;
use query_builder::insert_statement::{IncompleteInsertStatement, Insert};
use super::nodes::Replace;
/// Creates a SQLite `INSERT OR REPLACE` statement. If a constraint violation
/// fails, SQLite will attempt to replace the offending row instead.
///
/// # Example
///
/// ```rust
/// # #[macro_use] extern crate diesel;
/// # include!("src/doctest_setup.rs");
/// #
/// # table! {
/// #     users {
/// #         id -> Integer,
/// #         name -> VarChar,
/// #     }
/// # }
/// #
/// # struct User<'a> {
/// #     id: i32,
/// #     name: &'a str,
/// # }
/// #
/// # impl_Insertable! {
/// #     (users)
/// #     struct User<'a> {
/// #         id: i32,
/// #         name: &'a str,
/// #     }
/// # }
/// #
/// # fn main() {
/// #     use self::users::dsl::*;
/// #     use self::diesel::{insert, insert_or_replace};
/// #     use self::diesel::sqlite::SqliteConnection;
/// #
/// #     let conn = SqliteConnection::establish(":memory:").unwrap();
/// #     conn.execute("CREATE TABLE users (id INTEGER PRIMARY KEY AUTOINCREMENT, name VARCHAR)").unwrap();
/// insert(&NewUser::new("Sean")).into(users).execute(&conn).unwrap();
/// insert(&NewUser::new("Tess")).into(users).execute(&conn).unwrap();
///
/// let new_user = User { id: 1, name: "Jim" };
/// insert_or_replace(&new_user).into(users).execute(&conn).unwrap();
///
/// let names = users.select(name).order(id).load::<String>(&conn);
/// assert_eq!(Ok(vec!["Jim".into(), "Tess".into()]), names);
/// # }
/// ```
pub fn insert_or_replace<'a, T: ?Sized>(records: &'a T)
    -> IncompleteInsertStatement<&'a T, Or<Insert, Replace>>
{
    IncompleteInsertStatement::new(records, Or::new(Insert, Replace))
}
