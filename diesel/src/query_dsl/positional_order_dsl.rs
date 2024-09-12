use crate::backend::{Backend, DieselReserveSpecialization};
use crate::expression::helper_types::{Asc, Desc};
use crate::expression::Expression;
use crate::query_builder::{AstPass, QueryFragment, QueryId};
use crate::QueryResult;

/// The `positional_order_by` method
///
/// This trait is not yet part of Diesel's public API. It may change in the
/// future without a major version bump.
///
/// This trait exists as a stop-gap for users who need to order by column position
/// in their queries, so that they are not forced to drop entirely to raw SQL. The
/// arguments to `positional_order_by` are not checked, nor is the select statement
/// forced to be valid.
pub trait PositionalOrderDsl<Expr: PositionalOrderExpr> {
    /// The type returned by `.positional_order`
    type Output;

    /// See the trait documentation.
    fn positional_order_by(self, expr: Expr) -> Self::Output;
}

pub trait PositionalOrderExpr: Expression {}

impl PositionalOrderExpr for OrderColumn {}
impl<T: PositionalOrderExpr> PositionalOrderExpr for Asc<T> {}
impl<T: PositionalOrderExpr> PositionalOrderExpr for Desc<T> {}

#[derive(Debug, Clone, Copy, QueryId)]
pub struct OrderColumn(pub u32);

impl Expression for OrderColumn {
    type SqlType = crate::sql_types::Integer;
}

impl<DB> QueryFragment<DB> for OrderColumn
where
    DB: Backend + DieselReserveSpecialization,
{
    fn walk_ast<'b>(&'b self, mut pass: AstPass<'_, 'b, DB>) -> QueryResult<()> {
        pass.push_sql(&self.0.to_string());
        Ok(())
    }
}

impl From<u32> for OrderColumn {
    fn from(order: u32) -> Self {
        OrderColumn(order)
    }
}

macro_rules! impl_positional_order_expr_for_all_tuples {
    ($(
        $unused1:tt {
            $(($idx:tt) -> $T:ident, $unused2:ident, $unused3:tt,)+
        }
    )+) => {
        $(
            impl<$($T: PositionalOrderExpr),+> PositionalOrderExpr for ($($T,)+) { }
        )+
    };
}

diesel_derives::__diesel_for_each_tuple!(impl_positional_order_expr_for_all_tuples);
