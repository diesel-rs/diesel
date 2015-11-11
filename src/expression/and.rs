use query_builder::*;
use super::{Expression, SelectableExpression};
use types::Bool;

#[derive(Debug, Clone, Copy)]
pub struct And<T, U> {
    left: T,
    right: U,
}

impl<T, U> And<T, U> {
    pub fn new(left: T, right: U) -> Self {
        And {
            left: left,
            right: right,
        }
    }
}

impl<T, U> Expression for And<T, U> where
    T: Expression,
    U: Expression,
{
    type SqlType = Bool;

    fn to_sql<B: QueryBuilder>(&self, out: &mut B) -> BuildQueryResult {
        try!(self.left.to_sql(out));
        out.push_sql(" AND ");
        self.right.to_sql(out)
    }
}

impl<T, U, QS> SelectableExpression<QS> for And<T, U> where
    T: SelectableExpression<QS>,
    U: SelectableExpression<QS>,
{
}
