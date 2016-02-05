use backend::Backend;
use expression::*;
use query_builder::{QueryBuilder, QueryFragment, BuildQueryResult};
use types::Bool;

pub struct In<T, U> {
    left: T,
    values: Vec<U>,
}

impl<T, U> In<T, U> {
    pub fn new(left: T, values: Vec<U>) -> Self {
        In {
            left: left,
            values: values,
        }
    }
}

impl<T, U> Expression for In<T, U> where
    T: Expression,
    U: Expression<SqlType=T::SqlType>,
{
    type SqlType = Bool;
}

impl<T, U, QS> SelectableExpression<QS> for In<T, U> where
    In<T, U>: Expression,
    T: SelectableExpression<QS>,
    U: SelectableExpression<QS>,
{
}

impl<T, U> NonAggregate for In<T, U> where
    In<T, U>: Expression,
    T: NonAggregate,
    U: NonAggregate,
{
}

impl<T, U, DB> QueryFragment<DB> for In<T, U> where
    DB: Backend,
    T: QueryFragment<DB>,
    U: QueryFragment<DB>,
{
    fn to_sql(&self, out: &mut DB::QueryBuilder) -> BuildQueryResult {
        try!(self.left.to_sql(out));
        out.push_sql(" IN (");
        try!(self.values[0].to_sql(out));
        for value in self.values[1..].iter() {
            out.push_sql(", ");
            try!(value.to_sql(out));
        }
        out.push_sql(")");
        Ok(())
    }
}
