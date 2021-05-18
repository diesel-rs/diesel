use crate::backend::Backend;
use crate::expression::{
    AppearsOnTable, AsExpressionList, Expression, SelectableExpression, ValidGrouping,
};
use crate::query_builder::{AstPass, QueryFragment, QueryId};
use crate::sql_types;
use std::marker::PhantomData;

/// An ARRAY[...] literal.
#[derive(Debug, Clone, Copy, QueryId)]
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
/// #     Ok(())
/// # }
/// ```
pub fn array<ST, T>(elements: T) -> ArrayLiteral<T::Expression, ST>
where
    T: AsExpressionList<ST>,
{
    ArrayLiteral {
        elements: elements.as_expression_list(),
        _marker: PhantomData,
    }
}

impl<T, ST> Expression for ArrayLiteral<T, ST>
where
    T: Expression,
{
    type SqlType = sql_types::Array<ST>;
}

impl<T, ST, DB> QueryFragment<DB> for ArrayLiteral<T, ST>
where
    DB: Backend,
    for<'a> &'a T: QueryFragment<DB>,
{
    fn walk_ast(&self, mut out: AstPass<DB>) -> crate::result::QueryResult<()> {
        out.push_sql("ARRAY[");
        QueryFragment::walk_ast(&&self.elements, out.reborrow())?;
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
