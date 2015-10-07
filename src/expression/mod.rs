pub mod count;
mod max;
mod sql_literal;

pub mod dsl {
    pub use super::count::{count, count_star};
    pub use super::max::max;
}

pub use self::dsl::*;
pub use self::sql_literal::SqlLiteral;

use types::{self, NativeSqlType, ValuesToSql};
use persistable::AsBindParam;

pub trait Expression: Sized {
    type SqlType: NativeSqlType;

    fn to_sql(&self) -> String;

    fn binds(&self) -> Vec<Option<Vec<u8>>> {
        Vec::new()
    }

    fn eq<T>(self, other: T) -> Eq<Self, Bound<Self::SqlType, T>> where
        T: ValuesToSql<Self::SqlType> + AsBindParam
    {
        Eq { left: self, right: Bound::new(other) }
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

    fn to_sql(&self) -> String {
        format!("{} = {}", self.left.to_sql(), self.right.to_sql())
    }

    fn binds(&self) -> Vec<Option<Vec<u8>>> {
        let mut binds = self.left.binds();
        binds.append(&mut self.right.binds());
        binds
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
    fn new(item: U) -> Self {
        Bound { item: item, _marker: PhantomData }
    }
}

impl<T, U> Expression for Bound<T, U> where
    T: NativeSqlType,
    U: AsBindParam + ValuesToSql<T> + Debug,
{
    type SqlType = T;

    fn to_sql(&self) -> String {
        self.item.as_bind_param(&mut 1)
    }

    fn binds(&self) -> Vec<Option<Vec<u8>>> {
        self.item.values_to_sql()
            .ok().expect(&format!("Error serializing {:?}", self.item))
    }
}

impl<T, U, QS> SelectableExpression<QS> for Bound<T, U> where
    Bound<T, U>: Expression,
{
}
