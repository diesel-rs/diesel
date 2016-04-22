use expression::predicates::Or;
use query_builder::insert_statement::{IncompleteInsertStatement, Insert};
use super::nodes::Replace;

// FIXME: Replace this example with an actual running doctest once we have a
// more reasonable story for `impl Insertable` and friends without codegen
/// Creates a SQLite `INSERT OR REPLACE` statement. If a constraint violation
/// fails, SQLite will attempt to replace the offending row instead.
///
/// # Example
///
/// ```ignore
/// insert(&NewUser::new("Sean")).into(users).execute(&conn).unwrap();
/// insert(&NewUser::new("Tess")).into(users).execute(&conn).unwrap();
///
/// let new_user = User { id: 1, name: "Jim" };
/// insert_or_replace(&new_user).into(users).execute(&conn).unwrap();
///
/// let names = users.select(name).order(id).load::<String>(&conn);
/// assert_eq!(Ok(vec!["Jim".into(), "Tess".into()]), names);
/// ```
pub fn insert_or_replace<'a, T: ?Sized>(records: &'a T)
    -> IncompleteInsertStatement<&'a T, Or<Insert, Replace>>
{
    IncompleteInsertStatement::new(records, Or::new(Insert, Replace))
}
