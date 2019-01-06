use crate::expression::grouped::Grouped;
use crate::expression::operators::{And, Or};
use crate::expression::{AsExpression, Expression};
use crate::sql_types::Bool;

/// Methods present on boolean expressions
pub trait BoolExpressionMethods: Expression<SqlType = Bool> + Sized {
    /// Creates a SQL `AND` expression
    ///
    /// # Example
    ///
    /// ```
    /// # #[macro_use] extern crate diesel;
    /// # include!("../doctest_setup.rs");
    /// #
    /// # fn main() {
    /// #     run_test().unwrap();
    /// # }
    /// #
    /// # fn run_test() -> QueryResult<()> {
    /// #     use schema::animals::dsl::*;
    /// #     let connection = establish_connection();
    /// #
    /// diesel::insert_into(animals)
    ///     .values(&vec![
    ///         (species.eq("ferret"), legs.eq(4), name.eq("Freddy")),
    ///         (species.eq("ferret"), legs.eq(4), name.eq("Jack")),
    ///     ])
    ///     .execute(&connection)?;
    ///
    /// let data = animals.select((species, name))
    ///     .filter(species.eq("ferret").and(name.eq("Jack")))
    ///     .load(&connection)?;
    /// let expected = vec![
    ///     (String::from("ferret"), Some(String::from("Jack"))),
    /// ];
    /// assert_eq!(expected, data);
    /// #     Ok(())
    /// # }
    fn and<T: AsExpression<Bool>>(self, other: T) -> And<Self, T::Expression> {
        And::new(self.as_expression(), other.as_expression())
    }

    /// Creates a SQL `OR` expression
    ///
    /// The result will be wrapped in parenthesis, so that precedence matches
    /// that of your function calls. For example, `false.and(false.or(true))`
    /// will generate the SQL `FALSE AND (FALSE OR TRUE)`, which returns `false`
    ///
    /// # Example
    ///
    /// ```
    /// # #[macro_use] extern crate diesel;
    /// # include!("../doctest_setup.rs");
    /// #
    /// # fn main() {
    /// #     run_test().unwrap();
    /// # }
    /// #
    /// # fn run_test() -> QueryResult<()> {
    /// #     use schema::animals::dsl::*;
    /// #     let connection = establish_connection();
    /// #
    /// diesel::insert_into(animals)
    ///     .values(&vec![
    ///         (species.eq("ferret"), legs.eq(4), name.eq("Freddy")),
    ///         (species.eq("ferret"), legs.eq(4), name.eq("Jack")),
    ///     ])
    ///     .execute(&connection)?;
    ///
    /// let data = animals.select((species, name))
    ///     .filter(species.eq("ferret").or(name.eq("Jack")))
    ///     .load(&connection)?;
    /// let expected = vec![
    ///     (String::from("dog"), Some(String::from("Jack"))),
    ///     (String::from("ferret"), Some(String::from("Freddy"))),
    ///     (String::from("ferret"), Some(String::from("Jack"))),
    /// ];
    /// assert_eq!(expected, data);
    /// #     Ok(())
    /// # }
    fn or<T: AsExpression<Bool>>(self, other: T) -> Grouped<Or<Self, T::Expression>> {
        Grouped(Or::new(self, other.as_expression()))
    }
}

impl<T: Expression<SqlType = Bool>> BoolExpressionMethods for T {}
