use query_builder::*;
use super::{Expression, SelectableExpression, NonAggregate};
use types::{NativeSqlType, ToSql, IsNull};

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
    U: ToSql<T>,
{
    type SqlType = T;

    fn to_sql(&self, out: &mut QueryBuilder) -> BuildQueryResult {
        let mut bytes = Vec::new();
        match try!(self.item.to_sql(&mut bytes)) {
            IsNull::Yes => {
                out.push_bound_value(&self.tpe, None);
                Ok(())
            }
            IsNull::No => {
                out.push_bound_value(&self.tpe, Some(bytes));
                Ok(())
            }
        }
    }

    fn to_insert_sql(&self, out: &mut QueryBuilder) -> BuildQueryResult {
        let mut bytes = Vec::new();
        match try!(self.item.to_sql(&mut bytes)) {
            IsNull::Yes => {
                out.push_sql("DEFAULT");
                Ok(())
            }
            IsNull::No => {
                out.push_bound_value(&self.tpe, Some(bytes));
                Ok(())
            }
        }
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
