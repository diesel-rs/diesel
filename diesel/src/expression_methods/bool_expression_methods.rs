use expression::grouped::Grouped;
use expression::operators::{And, Or};
use expression::{AsExpression, Expression};
use sql_types::{Bool, Nullable};

/// Methods present on boolean expressions
pub trait BoolExpressionMethods: Expression + Sized {
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
    fn and<T: AsExpression<Self::SqlType>>(self, other: T) -> And<Self, T::Expression> {
        And::new(self, other.as_expression())
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
    fn or<T: AsExpression<Self::SqlType>>(self, other: T) -> Grouped<Or<Self, T::Expression>> {
        Grouped(Or::new(self, other.as_expression()))
    }
}

impl<T> BoolExpressionMethods for T
where
    T: Expression,
    T::SqlType: BoolOrNullableBool,
{
}

#[doc(hidden)]
/// Marker trait used to implement `BoolExpressionMethods` on the appropriate
/// types. Once coherence takes associated types into account, we can remove
/// this trait.
pub trait BoolOrNullableBool {}

impl BoolOrNullableBool for Bool {}
impl BoolOrNullableBool for Nullable<Bool> {}
