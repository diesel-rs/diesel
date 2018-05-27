use expression::count::*;
use super::SelectDsl;

/// Adds a simple `count` function to queries. Automatically implemented for all
/// types which implement `SelectDsl`.
///
/// # Example
///
/// ```rust
/// # #[macro_use] extern crate diesel;
/// # include!("../doctest_setup.rs");
/// #
/// # table! {
/// #     users {
/// #         id -> Integer,
/// #         name -> VarChar,
/// #     }
/// # }
/// #
/// # fn main() {
/// #     use self::users::dsl::*;
/// #     let connection = establish_connection();
/// let count = users.count().get_result(&connection);
/// assert_eq!(Ok(2), count);
/// # }
/// ```
pub trait CountDsl: SelectDsl<CountStar> + Sized {
    /// Get the count of a query. This is equivalent to `.select(count_star())`
    fn count(self) -> <Self as SelectDsl<CountStar>>::Output {
        self.select(count_star())
    }
}

impl<T: SelectDsl<CountStar>> CountDsl for T {}
