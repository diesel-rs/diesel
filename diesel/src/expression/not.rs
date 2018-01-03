use dsl::Not;
use expression::AsExpression;
use expression::grouped::Grouped;
use types::Bool;

/// Creates a SQL `NOT` expression
///
/// # Example
///
/// ```rust
/// # #[macro_use] extern crate diesel;
/// # include!("../doctest_setup.rs");
/// #
/// # fn main() {
/// #     use schema::users::dsl::*;
/// #     let connection = establish_connection();
/// use diesel::dsl::not;
///
/// let users_with_name = users.select(id).filter(name.eq("Sean"));
/// let users_not_with_name = users.select(id).filter(
///     not(name.eq("Sean")));
///
/// assert_eq!(Ok(1), users_with_name.first(&connection));
/// assert_eq!(Ok(2), users_not_with_name.first(&connection));
/// # }
/// ```
pub fn not<T: AsExpression<Bool>>(expr: T) -> Not<T> {
    super::operators::Not::new(Grouped(expr.as_expression()))
}
