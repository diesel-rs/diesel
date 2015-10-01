mod count;
mod max;

pub use self::count::{count, count_star};
pub use self::max::max;

use query_source::QuerySource;
use types::NativeSqlType;

pub trait Expression {
    type SqlType: NativeSqlType;

    fn to_sql(&self) -> String;
}

pub trait SelectableExpression<
    QS: QuerySource,
    Type: NativeSqlType = <Self as Expression>::SqlType,
>: Expression {
}

pub trait NonAggregate: Expression {
}
