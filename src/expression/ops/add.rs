use expression::{Expression, SelectableExpression, NonAggregate};
use query_builder::{QueryBuilder, BuildQueryResult};
use super::NumericSqlType;

pub struct Add<Lhs, Rhs> {
    lhs: Lhs,
    rhs: Rhs,
}

impl<Lhs, Rhs> Add<Lhs, Rhs> {
    pub fn new(left: Lhs, right: Rhs) -> Self {
        Add {
            lhs: left,
            rhs: right,
        }
    }
}

impl<Lhs, Rhs> Expression for Add<Lhs, Rhs> where
    Lhs: Expression,
    Lhs::SqlType: NumericSqlType,
    Rhs: Expression<SqlType=Lhs::SqlType>,
{
    type SqlType = Lhs::SqlType;

    fn to_sql<T: QueryBuilder>(&self, out: &mut T) -> BuildQueryResult {
        try!(self.lhs.to_sql(out));
        out.push_sql(" + ");
        self.rhs.to_sql(out)
    }
}

impl<Lhs, Rhs, QS> SelectableExpression<QS> for Add<Lhs, Rhs> where
    Lhs: SelectableExpression<QS>,
    Rhs: SelectableExpression<QS>,
    Add<Lhs, Rhs>: Expression,
{
}

impl<Lhs, Rhs> NonAggregate for Add<Lhs, Rhs> where
    Lhs: NonAggregate,
    Rhs: NonAggregate,
    Add<Lhs, Rhs>: Expression,
{
}

generic_addable_expr!(Add, A, B);
