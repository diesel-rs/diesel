use crate::backend::{Backend, DieselReserveSpecialization};
use crate::expression::helper_types::{Asc, Desc};
use crate::query_builder::combination_clause::CombinationClause;
use crate::query_builder::{AstPass, Query, QueryFragment, QueryId};
use crate::{QueryResult, RunQueryDsl};

/// This trait is not yet part of Diesel's public API. It may change in the
/// future without a major version bump.
///
/// This trait exists as a stop-gap for users who need to order by column position
/// in their queries, so that they are not forced to drop entirely to raw SQL. The
/// arguments to `positional_order_by` are not checked, nor is the select statement
/// forced to be valid.
pub trait PositionalOrderDsl<Expr: Order>: Sized {
    fn positional_order_by(self, expr: Expr) -> PositionalOrderClause<Self, Expr::Fragment> {
        PositionalOrderClause {
            source: self,
            expr: expr.into_fragment(),
        }
    }
}

#[derive(Debug, Clone, Copy, QueryId)]
pub struct PositionalOrderClause<Source, Expr> {
    source: Source,
    expr: Expr,
}

impl<Combinator, Rule, Source, Rhs, Expr: Order> PositionalOrderDsl<Expr>
    for CombinationClause<Combinator, Rule, Source, Rhs>
{
}

impl<Source, Expr> Query for PositionalOrderClause<Source, Expr>
where
    Source: Query,
{
    type SqlType = Source::SqlType;
}

impl<Source, Expr, Conn> RunQueryDsl<Conn> for PositionalOrderClause<Source, Expr> {}

impl<Source, Expr, DB> QueryFragment<DB> for PositionalOrderClause<Source, Expr>
where
    DB: Backend + DieselReserveSpecialization,
    Source: QueryFragment<DB>,
    Expr: QueryFragment<DB>,
{
    fn walk_ast<'b>(&'b self, mut pass: AstPass<'_, 'b, DB>) -> QueryResult<()> {
        self.source.walk_ast(pass.reborrow())?;
        pass.push_sql(" ORDER BY ");
        self.expr.walk_ast(pass)
    }
}

#[derive(Debug, Clone, Copy, QueryId)]
pub struct OrderColumn(u32);

impl<DB: Backend> QueryFragment<DB> for OrderColumn {
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

pub trait IntoOrderColumn: Into<OrderColumn> {
    fn asc(self) -> Asc<OrderColumn> {
        Asc { expr: self.into() }
    }
    fn desc(self) -> Desc<OrderColumn> {
        Desc { expr: self.into() }
    }
}

impl<T> IntoOrderColumn for T where T: Into<OrderColumn> {}

pub trait Order: Copy {
    type Fragment;

    fn into_fragment(self) -> Self::Fragment;
}

impl<T: Into<OrderColumn> + Copy> Order for T {
    type Fragment = OrderColumn;

    fn into_fragment(self) -> Self::Fragment {
        self.into()
    }
}

impl Order for Asc<OrderColumn> {
    type Fragment = Asc<OrderColumn>;

    fn into_fragment(self) -> Self::Fragment {
        self
    }
}

impl Order for Desc<OrderColumn> {
    type Fragment = Desc<OrderColumn>;

    fn into_fragment(self) -> Self::Fragment {
        self
    }
}

macro_rules! impl_order_for_all_tuples {
    ($(
        $unused1:tt {
            $(($idx:tt) -> $T:ident, $unused2:ident, $unused3:tt,)+
        }
    )+) => {
        $(
            impl<$($T: Order),+> Order for ($($T,)+) {
                type Fragment = ($(<$T as Order>::Fragment,)+);

                fn into_fragment(self) -> Self::Fragment {
                    ($(self.$idx.into_fragment(),)+)
                }
            }
        )+
    };
}

diesel_derives::__diesel_for_each_tuple!(impl_order_for_all_tuples);
