use super::{Expression, SelectableExpression};
use types::{SqlOrd, NativeSqlType};

pub fn max<ST, T>(t: T) -> Max<T> where
    ST: NativeSqlType + SqlOrd,
    T: Expression<SqlType=ST>,
{
    Max {
        target: t,
    }
}

pub struct Max<T: Expression> {
    target: T,
}

impl<T: Expression> Expression for Max<T> {
    type SqlType = T::SqlType;

    fn to_sql(&self) -> String {
        format!("MAX({})", self.target.to_sql())
    }
}

impl<T: Expression, QS> SelectableExpression<QS> for Max<T> {
}
