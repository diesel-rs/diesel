use query_builder::{QueryBuilder, BuildQueryResult};
use super::{Expression, SelectableExpression};

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
