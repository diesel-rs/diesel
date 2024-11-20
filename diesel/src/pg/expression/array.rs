use crate::expression::subselect::Subselect;
use crate::expression::{
    AppearsOnTable, AsExpressionList, Expression, SelectableExpression, TypedExpressionType,
    ValidGrouping,
};
use crate::pg::Pg;
use crate::query_builder::combination_clause::CombinationClause;
use crate::query_builder::{
    AstPass, BoxedSelectStatement, QueryFragment, QueryId, SelectQuery, SelectStatement,
};
use crate::sql_types::{self, SqlType};
use std::marker::PhantomData;

/// Creates an `ARRAY[...]` expression.
///
/// The argument should be a tuple of expressions which can be represented by the
/// same SQL type.
///
/// # Examples
///
/// ```rust
/// # include!("../../doctest_setup.rs");
/// #
/// # fn main() {
/// #     run_test().unwrap();
/// # }
/// #
/// # fn run_test() -> QueryResult<()> {
/// #     use schema::users::dsl::*;
/// #     use diesel::dsl::array;
/// #     use diesel::sql_types::Integer;
/// #     let connection = &mut establish_connection();
/// let ints = diesel::select(array::<Integer, _>((1, 2)))
///     .get_result::<Vec<i32>>(connection)?;
/// assert_eq!(vec![1, 2], ints);
///
/// let ids = users.select(array((id, id * 2)))
///     .get_results::<Vec<i32>>(connection)?;
/// let expected = vec![
///     vec![1, 2],
///     vec![2, 4],
/// ];
/// assert_eq!(expected, ids);
///
/// let ids = diesel::select(array(users.select(id)))
///     .get_results::<Vec<i32>>(connection)?;
/// assert_eq!(vec![1, 2], ids);
/// #     Ok(())
/// # }
/// ```
#[cfg(feature = "postgres_backend")]
pub fn array<ST, T>(elements: T) -> <T as IntoArrayExpression<ST>>::ArrayExpression
where
    T: IntoArrayExpression<ST>,
    ST: SqlType + TypedExpressionType,
{
    elements.into_array_expression()
}

pub trait IntoArrayExpression<ST: SqlType + TypedExpressionType> {
    /// Type of the expression returned by [AsArrayExpression::as_in_expression]
    type ArrayExpression: Expression<SqlType = sql_types::Array<ST>>;

    /// Construct the diesel query dsl representation of
    /// the `ARRAY (values)` clause for the given type
    fn into_array_expression(self) -> Self::ArrayExpression;
}

impl<ST, T> IntoArrayExpression<ST> for T
where
    T: AsExpressionList<ST>,
    ST: SqlType + TypedExpressionType,
    T::Expression: Expression<SqlType = ST>,
{
    type ArrayExpression = ArrayLiteral<T::Expression, ST>;

    fn into_array_expression(self) -> Self::ArrayExpression {
        ArrayLiteral {
            elements: self.as_expression_list(),
            _marker: PhantomData,
        }
    }
}

/// An ARRAY[...] literal.
#[derive(Debug, Clone, Copy, QueryId)]
pub struct ArrayLiteral<T, ST> {
    elements: T,
    _marker: PhantomData<ST>,
}

impl<T, ST> Expression for ArrayLiteral<T, ST>
where
    ST: 'static,
    T: Expression,
{
    type SqlType = sql_types::Array<ST>;
}

impl<T, ST> QueryFragment<Pg> for ArrayLiteral<T, ST>
where
    T: QueryFragment<Pg>,
{
    fn walk_ast<'b>(&'b self, mut out: AstPass<'_, 'b, Pg>) -> crate::result::QueryResult<()> {
        out.push_sql("ARRAY[");
        QueryFragment::walk_ast(&self.elements, out.reborrow())?;
        out.push_sql("]");
        Ok(())
    }
}

impl<T, ST, QS> SelectableExpression<QS> for ArrayLiteral<T, ST>
where
    T: SelectableExpression<QS>,
    ArrayLiteral<T, ST>: AppearsOnTable<QS>,
{
}

impl<T, ST, QS> AppearsOnTable<QS> for ArrayLiteral<T, ST>
where
    T: AppearsOnTable<QS>,
    ArrayLiteral<T, ST>: Expression,
{
}

impl<T, ST, GB> ValidGrouping<GB> for ArrayLiteral<T, ST>
where
    T: ValidGrouping<GB>,
{
    type IsAggregate = T::IsAggregate;
}

impl<ST, F, S, D, W, O, LOf, G, H, LC> IntoArrayExpression<ST>
    for SelectStatement<F, S, D, W, O, LOf, G, H, LC>
where
    ST: SqlType + TypedExpressionType,
    ArraySubselect<Self, ST>: Expression<SqlType = sql_types::Array<ST>>,
    Self: SelectQuery<SqlType = ST>,
{
    type ArrayExpression = ArraySubselect<Self, ST>;

    fn into_array_expression(self) -> Self::ArrayExpression {
        ArraySubselect::new(self)
    }
}

impl<'a, ST, QS, DB, GB> IntoArrayExpression<ST> for BoxedSelectStatement<'a, ST, QS, DB, GB>
where
    ST: SqlType + TypedExpressionType,
    ArraySubselect<BoxedSelectStatement<'a, ST, QS, DB, GB>, ST>:
        Expression<SqlType = sql_types::Array<ST>>,
{
    type ArrayExpression = ArraySubselect<Self, ST>;

    fn into_array_expression(self) -> Self::ArrayExpression {
        ArraySubselect::new(self)
    }
}

impl<ST, Combinator, Rule, Source, Rhs> IntoArrayExpression<ST>
    for CombinationClause<Combinator, Rule, Source, Rhs>
where
    ST: SqlType + TypedExpressionType,
    Self: SelectQuery<SqlType = ST>,
    ArraySubselect<Self, ST>: Expression<SqlType = sql_types::Array<ST>>,
{
    type ArrayExpression = ArraySubselect<Self, ST>;

    fn into_array_expression(self) -> Self::ArrayExpression {
        ArraySubselect::new(self)
    }
}

/// An ARRAY(...) subselect.
#[derive(Debug, Clone, Copy, QueryId)]
pub struct ArraySubselect<T, ST> {
    elements: Subselect<T, ST>,
}

impl<T, ST> ArraySubselect<T, ST> {
    pub(crate) fn new(elements: T) -> Self {
        Self {
            elements: Subselect::new(elements),
        }
    }
}

impl<T, ST> Expression for ArraySubselect<T, ST>
where
    ST: 'static,
    T: Expression,
{
    type SqlType = sql_types::Array<ST>;
}

impl<T, ST> QueryFragment<Pg> for ArraySubselect<T, ST>
where
    T: QueryFragment<Pg>,
{
    fn walk_ast<'b>(&'b self, mut out: AstPass<'_, 'b, Pg>) -> crate::result::QueryResult<()> {
        out.push_sql("ARRAY[");
        QueryFragment::walk_ast(&self.elements, out.reborrow())?;
        out.push_sql("]");
        Ok(())
    }
}

impl<T, ST, QS> SelectableExpression<QS> for ArraySubselect<T, ST>
where
    T: SelectableExpression<QS>,
    ArraySubselect<T, ST>: AppearsOnTable<QS>,
{
}

impl<T, ST, QS> AppearsOnTable<QS> for ArraySubselect<T, ST>
where
    T: AppearsOnTable<QS>,
    ArraySubselect<T, ST>: Expression,
{
}

impl<T, ST, GB> ValidGrouping<GB> for ArraySubselect<T, ST>
where
    T: ValidGrouping<GB>,
{
    type IsAggregate = T::IsAggregate;
}
