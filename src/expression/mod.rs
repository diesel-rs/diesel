pub mod count;
mod max;
mod sql_literal;

pub mod dsl {
    pub use super::count::{count, count_star};
    pub use super::max::max;
}

pub use self::dsl::*;
pub use self::sql_literal::SqlLiteral;

use persistable::AsBindParam;
use query_builder::{QueryBuilder, BuildQueryResult};
use types::{self, NativeSqlType, ValuesToSql};

pub trait Expression: Sized {
    type SqlType: NativeSqlType;

    fn to_sql<T: QueryBuilder>(&self, out: &mut T) -> BuildQueryResult;

    fn eq<T: AsExpression<Self::SqlType>>(self, other: T) -> Eq<Self, T::Expression> {
        Eq { left: self, right: other.as_expression() }
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

pub struct Eq<T, U> {
    left: T,
    right: U,
}

impl<T, U> Expression for Eq<T, U> where
    T: Expression,
    U: Expression,
{
    type SqlType = types::Bool;

    fn to_sql<B: QueryBuilder>(&self, out: &mut B) -> BuildQueryResult {
        try!(self.left.to_sql(out));
        out.push_sql(" = ");
        try!(self.right.to_sql(out));
        Ok(())
    }
}

impl<T, U, QS> SelectableExpression<QS> for Eq<T, U> where
    T: SelectableExpression<QS>,
    U: SelectableExpression<QS>,
{
}

use std::marker::PhantomData;
use std::fmt::Debug;

pub struct Bound<T, U> {
    item: U,
    _marker: PhantomData<T>,
}

impl<T, U> Bound<T, U> {
    pub fn new(item: U) -> Self {
        Bound { item: item, _marker: PhantomData }
    }
}

impl<T, U> Expression for Bound<T, U> where
    T: NativeSqlType,
    U: AsBindParam + ValuesToSql<T> + Debug,
{
    type SqlType = T;

    fn to_sql<B: QueryBuilder>(&self, out: &mut B) -> BuildQueryResult {
        self.item.values_to_sql().map(|mut values| {
            out.push_bound_value(values.pop().unwrap());
        })
    }
}

impl<T, U, QS> SelectableExpression<QS> for Bound<T, U> where
    Bound<T, U>: Expression,
{
}
