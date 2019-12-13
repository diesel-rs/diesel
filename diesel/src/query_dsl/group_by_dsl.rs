use crate::expression::Expression;
use crate::query_builder::{AsQuery, Query};
use crate::query_source::Table;

/// This trait is not yet part of Diesel's public API. It may change in the
/// future without a major version bump.
///
/// This trait exists as a stop-gap for users who need to use `GROUP BY` in
/// their queries, so that they are not forced to drop entirely to raw SQL. The
/// arguments to `group_by` are not checked, nor is the select statement
/// forced to be valid.
///
/// Since Diesel otherwise assumes that you have no `GROUP BY` clause (which
/// would mean that mixing an aggregate and non aggregate expression in the same
/// query is an error), you may need to use `sql` for your select clause.
pub trait GroupByDsl<Expr: Expression> {
    /// The type returned by `.group_by`
    type Output: Query;

    /// See the trait documentation.
    fn group_by(self, expr: Expr) -> Self::Output;
}

impl<T, Expr> GroupByDsl<Expr> for T
where
    Expr: Expression,
    T: Table + AsQuery,
    T::Query: GroupByDsl<Expr>,
{
    type Output = <T::Query as GroupByDsl<Expr>>::Output;

    fn group_by(self, expr: Expr) -> Self::Output {
        self.as_query().group_by(expr)
    }
}
