use crate::dsl;
use crate::expression::array_comparison::{AsInExpression, InExpression};
use crate::expression::subselect::Subselect;
use crate::expression::{
    AppearsOnTable, AsExpression, Expression, SelectableExpression, TypedExpressionType,
    ValidGrouping,
};
use crate::pg::Pg;
use crate::query_builder::combination_clause::CombinationClause;
use crate::query_builder::{
    AstPass, BoxedSelectStatement, QueryFragment, QueryId, SelectQuery, SelectStatement,
};
use crate::sql_types::{self, SqlType};
use std::marker::PhantomData;

/// Creates an `ARRAY[e1, e2, ...]` or `ARRAY(subselect)` expression.
///
/// The argument should be a tuple of expressions which can be represented by the
/// same SQL type, or a subquery.
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
/// let ints = diesel::select(array::<Integer, _>((1, 2))).get_result::<Vec<i32>>(connection)?;
/// assert_eq!(vec![1, 2], ints);
///
/// let ids = users
///     .select(array((id, id * 2)))
///     .get_results::<Vec<i32>>(connection)?;
/// let expected = vec![vec![1, 2], vec![2, 4]];
/// assert_eq!(expected, ids);
///
/// let ids = diesel::select(array(users.select(id))).first::<Vec<i32>>(connection)?;
/// assert_eq!(vec![1, 2], ids);
/// #     Ok(())
/// # }
/// ```
pub fn array<ST, T>(elements: T) -> dsl::array<ST, T>
where
    T: IntoArrayExpression<ST>,
    ST: SqlType + TypedExpressionType,
{
    elements.into_array_expression()
}

/// Return type of [`array(tuple_or_subselect)`](super::dsl::array())
#[allow(non_camel_case_types)]
#[cfg(feature = "postgres_backend")]
pub type array<ST, T> = <T as IntoArrayExpression<ST>>::ArrayExpression;

/// Trait for types which can be converted into an expression of type `Array`
///
/// This includes tuples of expressions with the same SQL type, and subselects with a single column.
#[diagnostic::on_unimplemented(
    message = "cannot convert `{Self}` into an expression of type `Array<{ST}>`",
    note = "`the trait bound `{Self}: IntoArrayExpression<{ST}>` is not satisfied. \
        (`AsExpressionList` is a deprecated trait alias for `IntoArrayExpression`)"
)]
pub trait IntoArrayExpression<ST: SqlType + TypedExpressionType> {
    /// Type of the expression returned by [IntoArrayExpression::into_array_expression]
    type ArrayExpression: Expression<SqlType = sql_types::Array<ST>>;

    /// Construct the diesel query dsl representation of
    /// the `ARRAY (values)` clause for the given type
    fn into_array_expression(self) -> Self::ArrayExpression;
}

/// Implement as a no-op for expressions that were already arrays (that is, don't wrap in ARRAY[]).
impl<ST, T> IntoArrayExpression<ST> for T
where
    T: AsExpression<sql_types::Array<ST>>,
    ST: SqlType + TypedExpressionType + 'static,
{
    type ArrayExpression = <T as AsExpression<sql_types::Array<ST>>>::Expression;

    fn into_array_expression(self) -> Self::ArrayExpression {
        <T as AsExpression<sql_types::Array<ST>>>::as_expression(self)
    }
}

// This has to be implemented for each tuple directly because an intermediate trait would cause
// conflicting impls. (Compiler says people could implement AsExpression<CustomSqlType> for
// SelectStatement<CustomType, ...>)
// This is not implemented with other tuple impls because this is feature-flagged by
// `postgres-backend`
macro_rules! tuple_impls {
    ($(
        $Tuple:tt {
            $(($idx:tt) -> $T:ident, $ST:ident, $TT:ident,)+
        }
    )+) => {
        $(
            impl<$($T,)+ ST> IntoArrayExpression<ST> for ($($T,)+) where
                $($T: AsExpression<ST>,)+
                ST: SqlType + TypedExpressionType,
            {
                type ArrayExpression = ArrayLiteral<($($T::Expression,)+), ST>;

                fn into_array_expression(self) -> Self::ArrayExpression {
                    ArrayLiteral {
                        elements: ($(self.$idx.as_expression(),)+),
                        _marker: PhantomData,
                    }
                }
            }
        )+
    }
}

diesel_derives::__diesel_for_each_tuple!(tuple_impls);

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

impl<T, ST> InExpression for ArrayLiteral<T, ST>
where
    Self: Expression<SqlType = sql_types::Array<ST>>,
    ST: SqlType,
{
    type SqlType = ST;

    fn is_empty(&self) -> bool {
        false
    }

    fn is_array(&self) -> bool {
        // we want to use the `= ANY(_)` syntax
        false
    }
}

impl<T, ST> AsInExpression<ST> for ArrayLiteral<T, ST>
where
    Self: Expression<SqlType = sql_types::Array<ST>>,
    ST: SqlType,
{
    type InExpression = Self;

    fn as_in_expression(self) -> Self::InExpression {
        self
    }
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
    subquery: Subselect<T, ST>,
}

impl<T, ST> ArraySubselect<T, ST> {
    pub(crate) fn new(elements: T) -> Self {
        Self {
            subquery: Subselect::new(elements),
        }
    }
}

impl<T, ST> Expression for ArraySubselect<T, ST>
where
    ST: 'static,
    Subselect<T, ST>: Expression<SqlType = ST>,
{
    type SqlType = sql_types::Array<ST>;
}

impl<T, ST> QueryFragment<Pg> for ArraySubselect<T, ST>
where
    Subselect<T, ST>: QueryFragment<Pg>,
{
    fn walk_ast<'b>(&'b self, mut out: AstPass<'_, 'b, Pg>) -> crate::result::QueryResult<()> {
        out.push_sql("ARRAY(");
        QueryFragment::walk_ast(&self.subquery, out.reborrow())?;
        out.push_sql(")");
        Ok(())
    }
}

impl<T, ST, QS> SelectableExpression<QS> for ArraySubselect<T, ST>
where
    Subselect<T, ST>: SelectableExpression<QS>,
    ArraySubselect<T, ST>: AppearsOnTable<QS>,
{
}

impl<T, ST, QS> AppearsOnTable<QS> for ArraySubselect<T, ST>
where
    Subselect<T, ST>: AppearsOnTable<QS>,
    ArraySubselect<T, ST>: Expression,
{
}

impl<T, ST, GB> ValidGrouping<GB> for ArraySubselect<T, ST>
where
    Subselect<T, ST>: ValidGrouping<GB>,
{
    type IsAggregate = <Subselect<T, ST> as ValidGrouping<GB>>::IsAggregate;
}

impl<T, ST> InExpression for ArraySubselect<T, ST>
where
    Self: Expression<SqlType = sql_types::Array<ST>>,
    ST: SqlType,
{
    type SqlType = ST;

    fn is_empty(&self) -> bool {
        false
    }

    fn is_array(&self) -> bool {
        // we want to use the `= ANY(_)` syntax
        false
    }
}

impl<T, ST> AsInExpression<ST> for ArraySubselect<T, ST>
where
    Self: Expression<SqlType = sql_types::Array<ST>>,
    ST: SqlType,
{
    type InExpression = Self;

    fn as_in_expression(self) -> Self::InExpression {
        self
    }
}
