use crate::dsl;
use crate::expression::Expression;
use crate::expression::TypedExpressionType;
use crate::expression::ValidGrouping;
use crate::query_builder::FromClause;
use crate::query_builder::{AsQuery, SelectStatement};
use crate::query_source::Table;

/// The `group_by` method
///
/// This trait should not be relied on directly by most apps. Its behavior is
/// provided by [`QueryDsl`]. However, you may need a where clause on this trait
/// to call `group_by` from generic code.
///
/// [`QueryDsl`]: crate::QueryDsl
pub trait GroupByDsl<Expr: Expression> {
    /// The type returned by `.group_by`
    type Output;

    /// See the trait documentation.
    fn group_by(self, expr: Expr) -> dsl::GroupBy<Self, Expr>;
}

impl<T, Expr> GroupByDsl<Expr> for T
where
    Expr: Expression,
    T: Table + AsQuery<Query = SelectStatement<FromClause<T>>>,
    T::DefaultSelection: Expression<SqlType = T::SqlType> + ValidGrouping<()>,
    T::SqlType: TypedExpressionType,
    T::Query: GroupByDsl<Expr>,
{
    type Output = dsl::GroupBy<SelectStatement<FromClause<T>>, Expr>;

    fn group_by(self, expr: Expr) -> dsl::GroupBy<Self, Expr> {
        self.as_query().group_by(expr)
    }
}
