use super::{UpdateTarget, IncompleteUpdateStatement};
use super::delete_statement::DeleteStatement;

pub fn update<T: UpdateTarget>(source: T) -> IncompleteUpdateStatement<T> {
    IncompleteUpdateStatement::new(source)
}

/// Creates a delete statement. Will delete the records in the given set.
/// Because this function has a very generic name, it is not exported by
/// default.
///
/// # Examples
///
/// ### Deleting a single record:
///
/// ```rust
/// # #[macro_use] extern crate yaqb;
/// # include!("src/doctest_setup.rs");
/// #
/// # table! {
/// #     users {
/// #         id -> Serial,
/// #         name -> VarChar,
/// #     }
/// # }
/// #
/// # fn main() {
/// #     use self::users::dsl::*;
/// #     use yaqb::query_builder::delete;
/// #     let connection = establish_connection();
/// #     let get_count = || users.count().first::<i64>(&connection).unwrap().unwrap();
/// let old_count = get_count();
/// let command = delete(users.filter(id.eq(1)));
/// connection.execute_returning_count(&command).unwrap();
/// assert_eq!(old_count - 1, get_count());
/// # }
/// ```
///
/// ### Deleting a whole table:
///
/// ```rust
/// # #[macro_use] extern crate yaqb;
/// # include!("src/doctest_setup.rs");
/// #
/// # table! {
/// #     users {
/// #         id -> Serial,
/// #         name -> VarChar,
/// #     }
/// # }
/// #
/// # fn main() {
/// #     use self::users::dsl::*;
/// #     use yaqb::query_builder::delete;
/// #     let connection = establish_connection();
/// #     let get_count = || users.count().first::<i64>(&connection).unwrap().unwrap();
/// connection.execute_returning_count(&delete(users)).unwrap();
/// assert_eq!(0, get_count());
/// # }
/// ```
pub fn delete<T: UpdateTarget>(source: T) -> DeleteStatement<T> {
    DeleteStatement::new(source)
}
