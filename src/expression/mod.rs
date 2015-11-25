#[macro_use]
pub mod ops;

pub mod aliased;
pub mod array_comparison;
pub mod bound;
pub mod count;
pub mod expression_methods;
pub mod extensions;
pub mod functions;
pub mod grouped;
pub mod helper_types;
pub mod max;
pub mod ordering;
pub mod predicates;
pub mod sql_literal;

pub mod dsl {
    pub use super::array_comparison::any;
    pub use super::count::{count, count_star};
    pub use super::functions::date_and_time::{now, date};
    pub use super::max::max;

    pub use super::extensions::*;
}

pub use self::dsl::*;
pub use self::sql_literal::SqlLiteral;

use query_builder::{QueryBuilder, BuildQueryResult};
use types::NativeSqlType;

pub trait Expression {
    type SqlType: NativeSqlType;

    fn to_sql(&self, out: &mut QueryBuilder) -> BuildQueryResult;
    fn to_insert_sql(&self, out: &mut QueryBuilder) -> BuildQueryResult {
        self.to_sql(out)
    }
}

impl<T: Expression + ?Sized> Expression for Box<T> {
    type SqlType = T::SqlType;

    fn to_sql(&self, out: &mut QueryBuilder) -> BuildQueryResult {
        Expression::to_sql(&**self, out)
    }

    fn to_insert_sql(&self, out: &mut QueryBuilder) -> BuildQueryResult {
        Expression::to_insert_sql(&**self, out)
    }
}

pub trait AsExpression<T: NativeSqlType> {
    type Expression: Expression<SqlType=T>;

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

impl<T: ?Sized, ST, QS> SelectableExpression<QS, ST> for Box<T> where
    T: SelectableExpression<QS, ST>,
    ST: NativeSqlType,
    Box<T>: Expression,
{
}

pub trait NonAggregate: Expression {
}

impl<T: NonAggregate + ?Sized> NonAggregate for Box<T> {
}

pub trait BoxableExpression<QS, ST: NativeSqlType>: Expression + SelectableExpression<QS, ST> + NonAggregate {
}

impl<QS, T, ST> BoxableExpression<QS, ST> for T where
    ST: NativeSqlType,
    T: Expression + SelectableExpression<QS, ST> + NonAggregate,
{
}
