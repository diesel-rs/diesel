use std::marker::PhantomData;
use backend::Backend;
use expression::{AppearsOnTable, Expression, NonAggregate, SelectableExpression};
use query_builder::{AstPass, QueryFragment};
use types;

/// An expression list which can be converted into a single Expression type.
pub trait IntoSingleTypeExpressionList<ST> {
    /// The Expression type the list of expressions can be converted into.
    type Expression;

    /// Convert the expression list into a single Expression.
    fn into_single_type_expression_list(self) -> Self::Expression;
}

/// An ARRAY[...] literal.
#[derive(Debug, Clone, Copy)]
pub struct ArrayLiteral<T, ST> {
    elements: T,
    _marker: PhantomData<ST>,
}

/// Creates an `ARRAY[...]` expression.
///
/// The argument should be a tuple of expressions which can be represented by the
/// same SQL type.
///
/// # Examples
///
/// ```rust
/// # #[macro_use] extern crate diesel;
/// # include!("../../doctest_setup.rs");
/// #
/// # table! {
/// #     users {
/// #         id -> Integer,
/// #         name -> VarChar,
/// #     }
/// # }
/// #
/// # fn main() {
/// #     use diesel::types;
/// #     use diesel::*;
/// #     use diesel::dsl::*;
/// #     use users::dsl::*;
/// #     let connection = establish_connection();
/// let ints = select(array::<types::Int4, _>((1, 2)))
///     .get_result::<Vec<i32>>(&connection);
/// // An array is returned as a Vec.
/// assert_eq!(Ok(vec![1, 2]), ints);
///
/// let ids = users.select(array((id, id * 2)))
///     .get_results::<Vec<i32>>(&connection);
/// assert_eq!(Ok(vec![vec![1, 2], vec![2, 4]]), ids);
/// # }
/// ```
pub fn array<ST, T>(elements: T) -> ArrayLiteral<T::Expression, ST>
where
    T: IntoSingleTypeExpressionList<ST>,
{
    ArrayLiteral {
        elements: elements.into_single_type_expression_list(),
        _marker: PhantomData,
    }
}

impl<T, ST> Expression for ArrayLiteral<T, ST>
where
    T: Expression,
{
    type SqlType = types::Array<ST>;
}

impl<T, ST, DB> QueryFragment<DB> for ArrayLiteral<T, ST>
where
    DB: Backend,
    for<'a> (&'a T): QueryFragment<DB>,
{
    fn walk_ast(&self, mut out: AstPass<DB>) -> ::result::QueryResult<()> {
        out.push_sql("ARRAY[");
        QueryFragment::walk_ast(&&self.elements, out.reborrow())?;
        out.push_sql("]");
        Ok(())
    }
}

impl_query_id!(ArrayLiteral<T, ST>);

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

impl<T, ST> NonAggregate for ArrayLiteral<T, ST>
where
    T: NonAggregate,
    ArrayLiteral<T, ST>: Expression,
{
}
