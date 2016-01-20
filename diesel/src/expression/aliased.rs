use expression::{Expression, NonAggregate, SelectableExpression};
use query_builder::*;
use query_source::*;

#[derive(Debug, Clone, Copy)]
pub struct Aliased<'a, Expr> {
    expr: Expr,
    alias: &'a str,
}

impl<'a, Expr> Aliased<'a, Expr> {
    pub fn new(expr: Expr, alias: &'a str) -> Self {
        Aliased {
            expr: expr,
            alias: alias,
        }
    }
}

pub struct FromEverywhere;

impl<'a, T> Expression for Aliased<'a, T> where
    T: Expression,
{
    type SqlType = T::SqlType;
}

impl<'a, T> QueryFragment for Aliased<'a, T> where
    T: QueryFragment,
{
    fn to_sql(&self, out: &mut QueryBuilder) -> BuildQueryResult {
        out.push_identifier(&self.alias)
    }
}

// FIXME This is incorrect, should only be selectable from WithQuerySource
impl<'a, T, QS> SelectableExpression<QS> for Aliased<'a, T> where
    Aliased<'a, T>: Expression,
{
}

impl<'a, T: Expression + QueryFragment> QuerySource for Aliased<'a, T> {
    fn from_clause(&self, out: &mut QueryBuilder) -> BuildQueryResult {
        try!(self.expr.to_sql(out));
        out.push_sql(" ");
        out.push_identifier(&self.alias)
    }
}

impl<'a, T> NonAggregate for Aliased<'a, T> where Aliased<'a, T>: Expression {
}
