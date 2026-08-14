use alloc::boxed::Box;

use crate::expression::{ValidGrouping, is_aggregate};
use alloc::sync::Arc;

simple_clause!(
    /// DSL node that represents that no order clause is set
    NoOrderClause,
    /// DSL node that represents that an order clause is set
    OrderClause,
    " ORDER BY "
);

impl<'a, DB, Expr> From<OrderClause<Expr>> for Option<Box<dyn QueryFragment<DB> + Send + 'a>>
where
    DB: Backend,
    Expr: QueryFragment<DB> + Send + 'a,
{
    fn from(order: OrderClause<Expr>) -> Self {
        Some(Box::new(order.0))
    }
}

impl<DB> From<NoOrderClause> for Option<Box<dyn QueryFragment<DB> + Send + '_>>
where
    DB: Backend,
{
    fn from(_: NoOrderClause) -> Self {
        None
    }
}

impl<'a, DB, Expr> From<OrderClause<Expr>> for Option<Arc<dyn QueryFragment<DB> + Send + Sync + 'a>>
where
    DB: Backend,
    Expr: QueryFragment<DB> + Send + Sync + 'a,
{
    fn from(order: OrderClause<Expr>) -> Self {
        Some(Arc::new(order.0))
    }
}

impl<DB> From<NoOrderClause> for Option<Arc<dyn QueryFragment<DB> + Send + Sync + '_>>
where
    DB: Backend,
{
    fn from(_: NoOrderClause) -> Self {
        None
    }
}

impl<GB> ValidGrouping<GB> for NoOrderClause {
    type IsAggregate = is_aggregate::Never;
}

impl<GB, Expr: ValidGrouping<GB>> ValidGrouping<GB> for OrderClause<Expr> {
    type IsAggregate = Expr::IsAggregate;
}
