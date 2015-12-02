use query_builder::{QueryBuilder, BuildQueryResult};
use super::{Expression, SelectableExpression};
use types::{SqlOrd, NativeSqlType};

/// Represents a SQL `MIN` function. This function can only take types which are
/// ordered.
pub fn min<ST, T>(t: T) -> Min<T> where
    ST: NativeSqlType + SqlOrd,
    T: Expression<SqlType=ST>,
{
    Min {
        target: t,
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Min<T: Expression> {
    target: T,
}

impl<T: Expression> Expression for Min<T> {
    type SqlType = T::SqlType;

    fn to_sql(&self, out: &mut QueryBuilder) -> BuildQueryResult {
        out.push_sql("MIN(");
        try!(self.target.to_sql(out));
        out.push_sql(")");
        Ok(())
    }
}

impl<T: Expression, QS> SelectableExpression<QS> for Min<T> {
}
