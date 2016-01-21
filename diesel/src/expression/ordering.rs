use query_builder::*;
use super::{Expression, SelectableExpression, NonAggregate};

pub struct Desc<T> {
    expr: T,
}

impl<T> Desc<T> {
    pub fn new(expr: T) -> Self {
        Desc {
            expr: expr,
        }
    }
}

impl<T> Expression for Desc<T> where
    T: Expression,
{
    type SqlType = ();
}

impl<T> QueryFragment for Desc<T> where
    T: QueryFragment,
{
    fn to_sql(&self, out: &mut QueryBuilder) -> BuildQueryResult {
        try!(self.expr.to_sql(out));
        out.push_sql(" DESC");
        Ok(())
    }
}

impl<T, QS> SelectableExpression<QS> for Desc<T> where
    Desc<T>: Expression,
    T: SelectableExpression<QS>,
{
}

impl<T: NonAggregate> NonAggregate for Desc<T> {}

pub struct Asc<T> {
    expr: T,
}

impl<T> Asc<T> {
    pub fn new(expr: T) -> Self {
        Asc {
            expr: expr,
        }
    }
}

impl<T> Expression for Asc<T> where
    T: Expression,
{
    type SqlType = ();
}

impl<T> QueryFragment for Asc<T> where
    T: QueryFragment,
{
    fn to_sql(&self, out: &mut QueryBuilder) -> BuildQueryResult {
        try!(self.expr.to_sql(out));
        out.push_sql(" ASC");
        Ok(())
    }
}

impl<T, QS> SelectableExpression<QS> for Asc<T> where
    Asc<T>: Expression,
    T: SelectableExpression<QS>,
{
}

impl<T: NonAggregate> NonAggregate for Asc<T> {}
