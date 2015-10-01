use query_source::QuerySource;
use super::{Expression, SelectableExpression};
use types::BigInt;

pub struct Count<T: Expression> {
    target: T,
}

pub fn count<T: Expression>(t: T) -> Count<T> {
    Count {
        target: t,
    }
}

impl<T: Expression> Expression for Count<T> {
    type SqlType = BigInt;

    fn to_sql(&self) -> String {
        format!("COUNT({})", self.target.to_sql())
    }
}

impl<T: Expression, QS: QuerySource> SelectableExpression<QS> for Count<T> {
}
