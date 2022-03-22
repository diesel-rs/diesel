//! Sqlite specific expression methods.

use super::operators::*;
use crate::dsl;
use crate::expression::grouped::Grouped;
use crate::expression::{AsExpression, Expression};
use crate::sql_types::SqlType;

/// Sqlite specific methods which are present on all expressions.
#[cfg(feature = "sqlite")]
pub trait SqliteExpressionMethods: Expression + Sized {
    /// Creates a Sqlite `IS` expression.
    ///
    /// The `IS` operator work like = except when one or both of the operands are NULL.
    /// In this case, if both operands are NULL, then the `IS` operator evaluates to true.
    /// If one operand is NULL and the other is not, then the `IS` operator evaluates to false.
    /// It is not possible for an `IS` expression to evaluate to NULL.
    ///
    /// # Example
    ///
    /// ```rust
    /// # include!("../../doctest_setup.rs");
    /// #
    /// # fn main() {
    /// #     run_test().unwrap();
    /// # }
    /// #
    /// # fn run_test() -> QueryResult<()> {
    /// #     use schema::animals::dsl::*;
    /// #     let connection = &mut establish_connection();
    /// let jack_is_a_dog = animals
    ///     .select(name)
    ///     .filter(species.is("dog"))
    ///     .get_results::<Option<String>>(connection)?;
    /// assert_eq!(vec![Some("Jack".to_string())], jack_is_a_dog);
    /// #     Ok(())
    /// # }
    /// ```
    fn is<T>(self, other: T) -> dsl::Is<Self, T>
    where
        Self::SqlType: SqlType,
        T: AsExpression<Self::SqlType>,
    {
        Grouped(Is::new(self, other.as_expression()))
    }

    /// Creates a Sqlite `IS NOT` expression.
    ///
    /// The `IS NOT` operator work like != except when one or both of the operands are NULL.
    /// In this case, if both operands are NULL, then the `IS NOT` operator evaluates to false.
    /// If one operand is NULL and the other is not, then the `IS NOT` operator is true.
    /// It is not possible for an `IS NOT` expression to evaluate to NULL.
    ///
    /// # Example
    ///
    /// ```rust
    /// # include!("../../doctest_setup.rs");
    /// #
    /// # fn main() {
    /// #     run_test().unwrap();
    /// # }
    /// #
    /// # fn run_test() -> QueryResult<()> {
    /// #     use schema::animals::dsl::*;
    /// #     let connection = &mut establish_connection();
    /// let jack_is_not_a_spider = animals
    ///     .select(name)
    ///     .filter(species.is_not("spider"))
    ///     .get_results::<Option<String>>(connection)?;
    /// assert_eq!(vec![Some("Jack".to_string())], jack_is_not_a_spider);
    /// #     Ok(())
    /// # }
    /// ```
    #[allow(clippy::wrong_self_convention)] // This is named after the sql operator
    fn is_not<T>(self, other: T) -> dsl::IsNot<Self, T>
    where
        Self::SqlType: SqlType,
        T: AsExpression<Self::SqlType>,
    {
        Grouped(IsNot::new(self, other.as_expression()))
    }
}

impl<T: Expression> SqliteExpressionMethods for T {}
