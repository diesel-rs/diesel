use query_builder::*;
use super::{Expression, SelectableExpression, NonAggregate};
use types::{NativeSqlType, ValuesToSql};

#[derive(Debug, Clone, Copy)]
pub struct Bound<T, U> {
    tpe: T,
    item: U,
}

impl<T: NativeSqlType, U> Bound<T, U> {
    pub fn new(item: U) -> Self {
        Bound { tpe: T::new(), item: item }
    }
}

impl<T, U> Expression for Bound<T, U> where
    T: NativeSqlType,
    U: ValuesToSql<T>,
{
    type SqlType = T;

    fn to_sql(&self, out: &mut QueryBuilder) -> BuildQueryResult {
        self.item.values_to_sql().map(|mut values| {
            out.push_bound_value(&self.tpe, values.pop().unwrap());
        })
    }

    fn to_insert_sql(&self, out: &mut QueryBuilder) -> BuildQueryResult {
        self.item.values_to_sql().map(|mut values| {
            match values.pop().unwrap() {
                values@Some(_) => out.push_bound_value(&self.tpe, values),
                None => out.push_sql("DEFAULT"),
            }
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
