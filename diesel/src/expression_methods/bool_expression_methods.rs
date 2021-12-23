use crate::dsl;
use crate::expression::grouped::Grouped;
use crate::expression::operators::{And, Or};
use crate::expression::{AsExpression, Expression, TypedExpressionType};
use crate::sql_types::{BoolOrNullableBool, SqlType};

/// Methods present on boolean expressions
pub trait BoolExpressionMethods: Expression + Sized {
    /// Creates a SQL `AND` expression
    ///
    /// # Example
    ///
    /// ```
    /// # include!("../doctest_setup.rs");
    /// #
    /// # fn main() {
    /// #     run_test().unwrap();
    /// # }
    /// #
    /// # fn run_test() -> QueryResult<()> {
    /// #     use schema::animals::dsl::*;
    /// #     let connection = &mut establish_connection();
    /// #
    /// diesel::insert_into(animals)
    ///     .values(&vec![
    ///         (species.eq("ferret"), legs.eq(4), name.eq("Freddy")),
    ///         (species.eq("ferret"), legs.eq(4), name.eq("Jack")),
    ///     ])
    ///     .execute(connection)?;
    ///
    /// let data = animals.select((species, name))
    ///     .filter(species.eq("ferret").and(name.eq("Jack")))
    ///     .load(connection)?;
    /// let expected = vec![
    ///     (String::from("ferret"), Some(String::from("Jack"))),
    /// ];
    /// assert_eq!(expected, data);
    /// #     Ok(())
    /// # }
    /// ```
    fn and<T, ST>(self, other: T) -> dsl::And<Self, T, ST>
    where
        Self::SqlType: SqlType,
        ST: SqlType + TypedExpressionType,
        T: AsExpression<ST>,
        And<Self, T::Expression>: Expression,
    {
        Grouped(And::new(self, other.as_expression()))
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
    /// # include!("../doctest_setup.rs");
    /// #
    /// # fn main() {
    /// #     run_test().unwrap();
    /// # }
    /// #
    /// # fn run_test() -> QueryResult<()> {
    /// #     use schema::animals::dsl::*;
    /// #     let connection = &mut establish_connection();
    /// #
    /// diesel::insert_into(animals)
    ///     .values(&vec![
    ///         (species.eq("ferret"), legs.eq(4), name.eq("Freddy")),
    ///         (species.eq("ferret"), legs.eq(4), name.eq("Jack")),
    ///     ])
    ///     .execute(connection)?;
    ///
    /// let data = animals.select((species, name))
    ///     .filter(species.eq("ferret").or(name.eq("Jack")))
    ///     .load(connection)?;
    /// let expected = vec![
    ///     (String::from("dog"), Some(String::from("Jack"))),
    ///     (String::from("ferret"), Some(String::from("Freddy"))),
    ///     (String::from("ferret"), Some(String::from("Jack"))),
    /// ];
    /// assert_eq!(expected, data);
    /// #     Ok(())
    /// # }
    /// ```
    fn or<T, ST>(self, other: T) -> dsl::Or<Self, T, ST>
    where
        Self::SqlType: SqlType,
        ST: SqlType + TypedExpressionType,
        T: AsExpression<ST>,
        Or<Self, T::Expression>: Expression,
    {
        Grouped(Or::new(self, other.as_expression()))
    }
}

impl<T> BoolExpressionMethods for T
where
    T: Expression,
    T::SqlType: BoolOrNullableBool,
{
}
