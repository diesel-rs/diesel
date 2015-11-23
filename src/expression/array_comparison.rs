use std::marker::PhantomData;

use query_builder::{QueryBuilder, BuildQueryResult};
use super::{AsExpression, Expression, SelectableExpression, NonAggregate};
use types::{Array, NativeSqlType};

pub fn any<ST, T>(vals: T) -> Any<T::Expression, ST> where
    ST: NativeSqlType,
    T: AsExpression<Array<ST>>,
{
    Any::new(vals.as_expression())
}

pub struct Any<Expr, ST> {
    expr: Expr,
    _marker: PhantomData<ST>,
}

impl<Expr, ST> Any<Expr, ST> {
    fn new(expr: Expr) -> Self {
        Any {
            expr: expr,
            _marker: PhantomData,
        }
    }
}

impl<Expr, ST> Expression for Any<Expr, ST> where
    ST: NativeSqlType,
    Expr: Expression<SqlType=Array<ST>>,
{
    type SqlType = ST;

    fn to_sql<B: QueryBuilder>(&self, out: &mut B) -> BuildQueryResult {
        out.push_sql("ANY(");
        try!(self.expr.to_sql(out));
        out.push_sql(")");
        Ok(())
    }
}

impl<Expr, ST, QS> SelectableExpression<QS> for Any<Expr, ST> where
    ST: NativeSqlType,
    Any<Expr, ST>: Expression,
    Expr: SelectableExpression<QS>,
{
}

impl<Expr, ST> NonAggregate for Any<Expr, ST> where
    Expr: NonAggregate,
    Any<Expr, ST>: Expression,
{
}
