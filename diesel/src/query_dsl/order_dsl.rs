use expression::Expression;
use query_source::Table;

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
///
/// # Examples
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
/// use self::users::dsl::{users, id, name};
///
/// let connection = establish_connection();
/// # connection.execute("DELETE FROM users").unwrap();
/// connection.execute("INSERT INTO users (name) VALUES ('Saul'), ('Steve'), ('Stan')").unwrap();
/// // load all users' names, ordered by their name descending
/// let ordered_names: Vec<String> = users.select(name).order(name.desc()).load(&connection).unwrap();
/// assert_eq!(vec![String::from("Steve"), String::from("Stan"), String::from("Saul")], ordered_names);
///
/// connection.execute("INSERT INTO users (name) VALUES ('Stan')").unwrap();
/// let ordered_name_id_pairs = users.select((name, id)).order((name.asc(), id.desc())).load(&connection).unwrap();
/// assert_eq!(vec![(String::from("Saul"), 3), (String::from("Stan"), 6), (String::from("Stan"), 5), (String::from("Steve"), 4)], ordered_name_id_pairs);
/// # }
/// ```
pub trait OrderDsl<Expr: Expression> {
    type Output;

    fn order(self, expr: Expr) -> Self::Output;
}

impl<T, Expr> OrderDsl<Expr> for T
where
    Expr: Expression,
    T: Table,
    T::Query: OrderDsl<Expr>,
{
    type Output = <T::Query as OrderDsl<Expr>>::Output;

    fn order(self, expr: Expr) -> Self::Output {
        self.as_query().order(expr)
    }
}
