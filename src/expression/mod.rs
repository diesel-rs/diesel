pub mod bound;
pub mod count;

mod eq;
mod max;
mod sql_literal;

pub mod dsl {
    pub use super::count::{count, count_star};
    pub use super::max::max;
}

pub use self::dsl::*;
pub use self::sql_literal::SqlLiteral;

use query_builder::{QueryBuilder, BuildQueryResult};
use self::eq::Eq;
use types::NativeSqlType;

pub trait Expression: Sized {
    type SqlType: NativeSqlType;

    fn to_sql<T: QueryBuilder>(&self, out: &mut T) -> BuildQueryResult;

    fn eq<T: AsExpression<Self::SqlType>>(self, other: T) -> Eq<Self, T::Expression> {
        Eq::new(self, other.as_expression())
    }
}

pub trait AsExpression<T: NativeSqlType> {
    type Expression: Expression;

    fn as_expression(self) -> Self::Expression;
}

impl<T: Expression> AsExpression<T::SqlType> for T {
    type Expression = Self;

    fn as_expression(self) -> Self {
        self
    }
}

pub trait SelectableExpression<
    QS,
    Type: NativeSqlType = <Self as Expression>::SqlType,
>: Expression {
}

pub trait NonAggregate: Expression {
}
