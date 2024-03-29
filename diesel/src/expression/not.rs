use crate::expression::grouped::Grouped;
use crate::expression::Expression;
use crate::helper_types;
use crate::sql_types::BoolOrNullableBool;

/// Creates a SQL `NOT` expression
///
/// # Example
///
/// ```rust
/// # include!("../doctest_setup.rs");
/// #
/// # fn main() {
/// #     use schema::users::dsl::*;
/// #     let connection = &mut establish_connection();
/// use diesel::dsl::not;
///
/// let users_with_name = users.select(id).filter(name.eq("Sean"));
/// let users_not_with_name = users.select(id).filter(
///     not(name.eq("Sean")));
///
/// assert_eq!(Ok(1), users_with_name.first(connection));
/// assert_eq!(Ok(2), users_not_with_name.first(connection));
/// # }
/// ```
pub fn not<T>(expr: T) -> helper_types::not<T>
where
    T: Expression,
    <T as Expression>::SqlType: BoolOrNullableBool,
{
    super::operators::Not::new(Grouped(expr))
}
