use std::marker::PhantomData;

use backend::Backend;
use query_builder::*;
use super::{Expression, NonAggregate, SelectableExpression};
use types::{HasSqlType, IsNull, ToSql};

#[derive(Debug, Clone, Copy)]
pub struct Bound<T, U> {
    item: U,
    _marker: PhantomData<T>,
}

impl<T, U> Bound<T, U> {
    pub fn new(item: U) -> Self {
        Bound {
            item: item,
            _marker: PhantomData,
        }
    }
}

impl<T, U> Expression for Bound<T, U> {
    type SqlType = T;
}

impl<T, U, DB> QueryFragment<DB> for Bound<T, U>
    where DB: Backend + HasSqlType<T>,
          U: ToSql<T, DB>,
{
    fn to_sql(&self, out: &mut DB::QueryBuilder) -> BuildQueryResult {
        let mut bytes = Vec::new();
        match try!(self.item.to_sql(&mut bytes)) {
            IsNull::Yes => {
                out.push_bound_value::<T>(None);
                Ok(())
            },
            IsNull::No => {
                out.push_bound_value::<T>(Some(bytes));
                Ok(())
            },
        }
    }
}

impl<T, U, QS> SelectableExpression<QS> for Bound<T, U> where Bound<T, U>: Expression, {}

impl<T, U> NonAggregate for Bound<T, U> where Bound<T, U>: Expression, {}
