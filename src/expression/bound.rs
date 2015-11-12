use persistable::AsBindParam;
use query_builder::*;
use std::fmt::Debug;
use std::marker::PhantomData;
use super::{Expression, SelectableExpression, NonAggregate};
use types::{NativeSqlType, ValuesToSql};

#[derive(Debug, Clone, Copy)]
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
            out.push_bound_value::<T>(values.pop().unwrap());
        })
    }
}

impl<T, U, QS> SelectableExpression<QS> for Bound<T, U> where
    Bound<T, U>: Expression,
{
}

impl<T, U> NonAggregate for Bound<T, U> where
    Bound<T, U>: Expression,
{
}
