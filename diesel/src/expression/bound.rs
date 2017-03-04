use std::marker::PhantomData;

use backend::Backend;
use query_builder::*;
use result::Error::SerializationError;
use result::QueryResult;
use super::*;
use types::{HasSqlType, ToSql, IsNull};

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

impl<T, U, DB> QueryFragment<DB> for Bound<T, U> where
    DB: Backend + HasSqlType<T>,
    U: ToSql<T, DB>,
{
    fn to_sql(&self, out: &mut DB::QueryBuilder) -> BuildQueryResult {
        out.push_bind_param();
        Ok(())
    }

    fn collect_binds(&self, out: &mut DB::BindCollector) -> QueryResult<()> {
        let mut bytes = Vec::new();
        let is_null = try!(self.item.to_sql(&mut bytes).map_err(SerializationError));
        match is_null {
            IsNull::Yes => {
                out.push_bound_value::<T>(None);
                Ok(())
            }
            IsNull::No => {
                out.push_bound_value::<T>(Some(bytes));
                Ok(())
            }
        }
    }

    fn is_safe_to_cache_prepared(&self) -> bool {
        true
    }
}

impl<T: QueryId, U> QueryId for Bound<T, U> {
    type QueryId = Bound<T::QueryId, ()>;

    fn has_static_query_id() -> bool {
        T::has_static_query_id()
    }
}

impl<T, U, QS> SelectableExpression<QS> for Bound<T, U> where
    Bound<T, U>: AppearsOnTable<QS>,
{
    type SqlTypeForSelect = T;
}

impl<T, U, QS> AppearsOnTable<QS> for Bound<T, U> where
    Bound<T, U>: Expression,
{
}

impl<T, U> NonAggregate for Bound<T, U> where
    Bound<T, U>: Expression,
{
}
