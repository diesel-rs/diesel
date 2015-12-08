use query_builder::{QueryBuilder, BuildQueryResult};
use super::{Expression, SelectableExpression};
use types::{BigInt, NativeSqlType};

/// Represents a SQL `SUM` function.
pub fn sum<ST, T>(t: T) -> Sum<T> where
    ST: NativeSqlType,
    T: Expression<SqlType=ST>,
{
    Sum {
        target: t,
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Sum<T: Expression> {
    target: T,
}

impl<T: Expression> Expression for Sum<T> {
    type SqlType = BigInt;

    fn to_sql(&self, out: &mut QueryBuilder) -> BuildQueryResult {
        out.push_sql("SUM(");
        try!(self.target.to_sql(out));
        out.push_sql(")");
        Ok(())
    }
}

impl<T: Expression, QS> SelectableExpression<QS> for Sum<T> {
}
