use query_builder::{QueryBuilder, BuildQueryResult};
use super::{Expression, SelectableExpression};
use types::BigInt;

pub fn count<T: Expression>(t: T) -> Count<T> {
    Count {
        target: t,
    }
}

pub fn count_star() -> CountStar {
    CountStar
}

pub struct Count<T: Expression> {
    target: T,
}

impl<T: Expression> Expression for Count<T> {
    type SqlType = BigInt;

    fn to_sql<B: QueryBuilder>(&self, out: &mut B) -> BuildQueryResult {
        out.push_sql("COUNT(");
        try!(self.target.to_sql(out));
        out.push_sql(")");
        Ok(())
    }
}

impl<T: Expression, QS> SelectableExpression<QS> for Count<T> {
}

pub struct CountStar;

impl Expression for CountStar {
    type SqlType = BigInt;

    fn to_sql<B: QueryBuilder>(&self, out: &mut B) -> BuildQueryResult {
        out.push_sql("COUNT(*)");
        Ok(())
    }
}

impl<QS> SelectableExpression<QS> for CountStar {
}
