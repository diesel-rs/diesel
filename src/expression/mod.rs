#[macro_use]
pub mod ops;

pub mod array_comparison;
pub mod bound;
pub mod count;
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
use self::grouped::Grouped;
use self::predicates::*;
use types::{self, NativeSqlType};

pub trait Expression {
    type SqlType: NativeSqlType;

    fn to_sql(&self, out: &mut QueryBuilder) -> BuildQueryResult;
    fn to_insert_sql(&self, out: &mut QueryBuilder) -> BuildQueryResult {
        self.to_sql(out)
    }

    fn eq<T: AsExpression<Self::SqlType>>(self, other: T) -> Eq<Self, T::Expression>
        where Self: Sized,
    {
        Eq::new(self, other.as_expression())
    }

    fn ne<T: AsExpression<Self::SqlType>>(self, other: T) -> NotEq<Self, T::Expression>
        where Self: Sized,
    {
        NotEq::new(self, other.as_expression())
    }

    fn gt<T: AsExpression<Self::SqlType>>(self, other: T) -> Gt<Self, T::Expression>
        where Self: Sized,
    {
        Gt::new(self, other.as_expression())
    }

    fn ge<T: AsExpression<Self::SqlType>>(self, other: T) -> GtEq<Self, T::Expression>
        where Self: Sized,
    {
        GtEq::new(self, other.as_expression())
    }

    fn lt<T: AsExpression<Self::SqlType>>(self, other: T) -> Lt<Self, T::Expression>
        where Self: Sized,
    {
        Lt::new(self, other.as_expression())
    }

    fn le<T: AsExpression<Self::SqlType>>(self, other: T) -> LtEq<Self, T::Expression>
        where Self: Sized,
    {
        LtEq::new(self, other.as_expression())
    }

    fn between<T: AsExpression<Self::SqlType>>(self, other: ::std::ops::Range<T>)
    -> Between<Self, And<T::Expression, T::Expression>>
        where Self: Sized,
    {
        Between::new(self, And::new(other.start.as_expression(), other.end.as_expression()))
    }

    fn not_between<T: AsExpression<Self::SqlType>>(self, other: ::std::ops::Range<T>)
    -> NotBetween<Self, And<T::Expression, T::Expression>>
        where Self: Sized,
    {
        NotBetween::new(self, And::new(other.start.as_expression(), other.end.as_expression()))
    }

    fn and<T: AsExpression<types::Bool>>(self, other: T) -> And<Self, T::Expression>
        where Self: Sized,
    {
        And::new(self.as_expression(), other.as_expression())
    }

    fn or<T: AsExpression<types::Bool>>(self, other: T) -> Grouped<Or<Self, T::Expression>>
        where Self: Sized,
    {
        Grouped(Or::new(self, other.as_expression()))
    }

    fn like<T: AsExpression<types::VarChar>>(self, other: T) -> Like<Self, T::Expression>
        where Self: Sized,
    {
        Like::new(self.as_expression(), other.as_expression())
    }

    fn not_like<T: AsExpression<types::VarChar>>(self, other: T) -> NotLike<Self, T::Expression>
        where Self: Sized,
    {
        NotLike::new(self.as_expression(), other.as_expression())
    }

    fn desc(self) -> ordering::Desc<Self>
        where Self: Sized,
    {
        ordering::Desc::new(self)
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

pub trait NonAggregate: Expression {
}
