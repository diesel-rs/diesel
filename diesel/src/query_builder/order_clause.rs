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
