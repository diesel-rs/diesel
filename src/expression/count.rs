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

    fn to_sql(&self) -> String {
        format!("COUNT({})", self.target.to_sql())
    }
}

impl<T: Expression, QS> SelectableExpression<QS> for Count<T> {
}

pub struct CountStar;

impl Expression for CountStar {
    type SqlType = BigInt;

    fn to_sql(&self) -> String {
        "COUNT(*)".to_string()
    }
}

impl<QS> SelectableExpression<QS> for CountStar {
}
