#[cfg(doc)]
use super::functions::aggregate_expressions::{
    AggregateExpressionMethods, WindowExpressionMethods,
};
use super::functions::declare_sql_function;
use super::{Expression, ValidGrouping};
use crate::backend::Backend;
use crate::internal::sql_functions::{
    FunctionFragment, IsWindowFunction, OverClause, WindowFunctionFragment,
};
use crate::query_builder::*;
use crate::result::QueryResult;
use crate::sql_types::{BigInt, DieselNumericOps, SingleValue, SqlType};

#[declare_sql_function]
extern "SQL" {
    /// Creates a SQL `COUNT` expression
    ///
    /// As with most bare functions, this is not exported by default. You can import
    /// it specifically as `diesel::dsl::count`, or glob import
    /// `diesel::dsl::*`
    ///
    /// ## Window Function Usage
    ///
    /// This function can be used as window function. See [`WindowExpressionMethods`] for details
    ///
    /// ## Aggregate Function Expression
    ///
    /// This function can be used as aggregate expression. See [`AggregateExpressionMethods`] for details.
    ///
    /// # Examples
    ///
    /// ## Normal function usage
    ///
    /// ```rust
    /// # include!("../doctest_setup.rs");
    /// # use diesel::dsl::*;
    /// #
    /// # fn main() {
    /// #     use schema::animals::dsl::*;
    /// #     let connection = &mut establish_connection();
    /// assert_eq!(Ok(1), animals.select(count(name)).first(connection));
    /// # }
    /// ```
    ///
    /// ## Window function
    ///
    /// ```rust
    /// # include!("../doctest_setup.rs");
    /// # use diesel::dsl::*;
    /// #
    /// # fn main() {
    /// #     use schema::animals::dsl::*;
    /// #     let connection = &mut establish_connection();
    /// assert_eq!(
    ///     Ok(1),
    ///     animals
    ///         .select(count(name).partition_by(id))
    ///         .first(connection)
    /// );
    /// # }
    /// ```
    ///
    /// ## Aggregate function expression
    ///
    /// ```rust
    /// # include!("../doctest_setup.rs");
    /// # use diesel::dsl::*;
    /// #
    /// # fn main() {
    /// #     use schema::animals::dsl::*;
    /// #     let connection = &mut establish_connection();
    /// assert_eq!(
    ///     Ok(1),
    ///     animals
    ///         .select(count(name).aggregate_distinct())
    ///         .first(connection)
    /// );
    /// # }
    /// ```
    #[aggregate]
    #[window]
    fn count<T: SqlType + SingleValue>(expr: T) -> BigInt;
}

/// Creates a SQL `COUNT(*)` expression
///
/// For selecting the count of a query, and nothing else, you can just call
/// [`count`](crate::query_dsl::QueryDsl::count())
/// on the query instead.
///
/// As with most bare functions, this is not exported by default. You can import
/// it specifically as `diesel::dsl::count_star`, or glob import
/// `diesel::dsl::*`
///
/// # Examples
///
/// ```rust
/// # include!("../doctest_setup.rs");
/// # use diesel::dsl::*;
/// #
/// # fn main() {
/// #     use schema::users::dsl::*;
/// #     let connection = &mut establish_connection();
/// assert_eq!(Ok(2), users.select(count_star()).first(connection));
/// # }
/// ```
pub fn count_star() -> CountStar {
    CountStar
}

#[derive(Debug, Clone, Copy, QueryId, DieselNumericOps, ValidGrouping)]
#[diesel(aggregate)]
#[doc(hidden)]
pub struct CountStar;

impl Expression for CountStar {
    type SqlType = BigInt;
}

impl<DB: Backend> FunctionFragment<DB> for CountStar {
    const FUNCTION_NAME: &'static str = "COUNT";

    fn walk_arguments<'b>(&'b self, mut out: AstPass<'_, 'b, DB>) -> QueryResult<()> {
        out.push_sql("*");
        Ok(())
    }
}

impl<DB: Backend> QueryFragment<DB> for CountStar {
    fn walk_ast<'b>(&'b self, mut out: AstPass<'_, 'b, DB>) -> QueryResult<()> {
        out.push_sql("COUNT(*)");
        Ok(())
    }
}

impl<Partition, Order, Frame, DB: Backend> WindowFunctionFragment<CountStar, DB>
    for OverClause<Partition, Order, Frame>
{
}

impl IsWindowFunction for CountStar {
    type ArgTypes = ();
}

impl_selectable_expression!(CountStar);

#[doc(hidden)]
#[cfg(all(feature = "with-deprecated", not(feature = "without-deprecated")))]
#[deprecated(note = "Use `AggregateExpressionMethods::aggregate_distinct` instead")]
pub fn count_distinct<T, E>(expr: E) -> CountDistinct<T, E::Expression>
where
    T: SqlType + SingleValue,
    E: crate::expression::AsExpression<T>,
{
    use crate::AggregateExpressionMethods;

    count(expr).aggregate_distinct()
}

#[doc(hidden)]
#[cfg(all(feature = "with-deprecated", not(feature = "without-deprecated")))]
pub type CountDistinct<T, E> = crate::dsl::AggregateDistinct<self::count<T, E>>;
