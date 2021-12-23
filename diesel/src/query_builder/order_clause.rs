simple_clause!(NoOrderClause, OrderClause, " ORDER BY ");

impl<'a, DB, Expr> From<OrderClause<Expr>> for Option<Box<dyn QueryFragment<DB> + Send + 'a>>
where
    DB: Backend,
    Expr: QueryFragment<DB> + Send + 'a,
{
    fn from(order: OrderClause<Expr>) -> Self {
        Some(Box::new(order.0))
    }
}

impl<'a, DB> From<NoOrderClause> for Option<Box<dyn QueryFragment<DB> + Send + 'a>>
where
    DB: Backend,
{
    fn from(_: NoOrderClause) -> Self {
        None
    }
}
