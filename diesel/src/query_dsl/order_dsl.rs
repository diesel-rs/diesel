use expression::Expression;
use query_builder::AsQuery;
use query_source::QuerySource;

/// Sets the order clause of a query. If there was already a order clause, it
/// will be overridden. The expression passed to `order` must actually be valid
/// for the query. See also:
/// [`.desc()`](../expression/expression_methods/global_expression_methods/trait.ExpressionMethods.html#method.desc)
/// and [`.asc()`](../expression/expression_methods/global_expression_methods/trait.ExpressionMethods.html#method.asc)
///
/// Ordering by multiple columns can be achieved by passing a tuple of those
/// columns.
///
/// This is automatically implemented for the various query builder types.
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
/// # fn main() {
/// #     use diesel::result::Error::NotFound;
/// let connection = establish_connection();
/// use self::users::dsl::*;
/// // load the users, ordered by their ID descending and then by their name descending.
/// // note: the id and name fields are imported from self::users::dsl::* and use the name of the columns in the DB, not in the struct.
/// users.order((id.desc(), name.desc())).load::<User>(&connection).unwrap();
/// # }
/// ```
pub trait OrderDsl<Expr: Expression>: AsQuery {
    type Output: AsQuery<SqlType=Self::SqlType>;

    fn order(self, expr: Expr) -> Self::Output;
}

impl<T, Expr, ST> OrderDsl<Expr> for T where
    Expr: Expression,
    T: QuerySource + AsQuery<SqlType=ST>,
    T::Query: OrderDsl<Expr, SqlType=ST>,
{
    type Output = <T::Query as OrderDsl<Expr>>::Output;

    fn order(self, expr: Expr) -> Self::Output {
        self.as_query().order(expr)
    }
}
