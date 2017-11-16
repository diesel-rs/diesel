use std::marker::PhantomData;
use expression::{AppearsOnTable, Expression, SelectableExpression};

pub trait IntoSingleTypeExpressionList<ST> {
    type Expression;

    fn into_single_type_expression_list(self) -> Self::Expression;
}

#[derive(Debug)]
pub struct Array<T, ST> {
    elements: T,
    _marker: PhantomData<ST>,
}

/// Creates an `ARRAY[...]` expression.  The argument should be a tuple of
/// expressions which can be represented by the same SQL type.
///
/// If the type can't be inferred, call `array::<SomeSqlType, _>((...))` to
/// specify it.
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
/// // The type of `id` is known so we don't have to specify the SQL type here.
/// let ids = users.select(array((id, id * 2)))
///     .get_results::<Vec<i32>>(&connection);
/// assert_eq!(Ok(vec![vec![1, 2], vec![2, 4]]), ids);
/// # }
/// ```
pub fn array<ST, T>(elements: T) -> Array<T::Expression, ST>
where
    T: IntoSingleTypeExpressionList<ST>,
{
    Array {
        elements: elements.into_single_type_expression_list(),
        _marker: PhantomData,
    }
}

impl<T, ST> Expression for Array<T, ST>
where
    T: Expression,
{
    type SqlType = ::pg::types::sql_types::Array<ST>;
}

use query_builder::{AstPass, QueryFragment};
use backend::Backend;

impl<T, ST, DB> QueryFragment<DB> for Array<T, ST>
where
    DB: Backend,
    for<'a> (&'a T): QueryFragment<DB>,
{
    fn walk_ast(&self, mut out: AstPass<DB>) -> ::result::QueryResult<()> {
        out.push_sql("ARRAY[");
        QueryFragment::walk_ast(&(&self.elements,), out.reborrow())?;
        out.push_sql("]");
        Ok(())
    }
}

impl_query_id!(Array<T, ST>);

impl<T, ST, QS> SelectableExpression<QS> for Array<T, ST>
where
    T: SelectableExpression<QS>,
    Array<T, ST>: AppearsOnTable<QS>,
{
}

impl<T, ST, QS> AppearsOnTable<QS> for Array<T, ST>
where
    T: AppearsOnTable<QS>,
    Array<T, ST>: Expression,
{
}

impl<T, ST> ::expression::NonAggregate for Array<T, ST>
where
    T: ::expression::NonAggregate,
    Array<T, ST>: Expression,
{
}
