use crate::dsl;
use crate::expression::TypedExpressionType;
use crate::expression::ValidGrouping;
use crate::query_builder::AsQuery;
use crate::query_builder::FromClause;
use crate::query_builder::SelectStatement;
use crate::query_source::Table;
use crate::Expression;

/// The `into_boxed` method
///
/// This trait should not be relied on directly by most apps. Its behavior is
/// provided by [`QueryDsl`]. However, you may need a where clause on this trait
/// to call `into_boxed` from generic code.
///
/// [`QueryDsl`]: crate::QueryDsl
#[diagnostic::on_unimplemented(
    message = "cannot box `{Self}` for backend `{DB}`",
    note = "this either means `{Self}` is no valid SQL for `{DB}`",
    note = "or this means `{Self}` uses clauses not supporting boxing like the `LOCKING` or `GROUP BY` clause"
)]
pub trait BoxedDsl<'a, DB> {
    /// The return type of `internal_into_boxed`
    type Output;

    /// See the trait documentation.
    fn internal_into_boxed(self) -> dsl::IntoBoxed<'a, Self, DB>;
}

#[diagnostic::do_not_recommend]
impl<'a, T, DB> BoxedDsl<'a, DB> for T
where
    T: Table + AsQuery<Query = SelectStatement<FromClause<T>>>,
    SelectStatement<FromClause<T>>: BoxedDsl<'a, DB>,
    T::DefaultSelection: Expression<SqlType = T::SqlType> + ValidGrouping<()>,
    T::SqlType: TypedExpressionType,
{
    type Output = dsl::IntoBoxed<'a, SelectStatement<FromClause<T>>, DB>;

    fn internal_into_boxed(self) -> Self::Output {
        self.as_query().internal_into_boxed()
    }
}
