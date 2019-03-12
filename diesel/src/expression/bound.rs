use std::marker::PhantomData;

use super::*;
use backend::Backend;
use query_builder::*;
use result::QueryResult;
use serialize::ToSql;
use sql_types::HasSqlType;

#[derive(Debug, Clone, Copy, DieselNumericOps)]
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
where
    DB: Backend + HasSqlType<T>,
    U: ToSql<T, DB>,
{
    fn walk_ast(&self, mut pass: AstPass<DB>) -> QueryResult<()> {
        pass.push_bind_param(&self.item)?;
        Ok(())
    }
}

impl<T: QueryId, U> QueryId for Bound<T, U> {
    type QueryId = Bound<T::QueryId, ()>;

    const HAS_STATIC_QUERY_ID: bool = T::HAS_STATIC_QUERY_ID;
}

impl<T, U, QS> SelectableExpression<QS> for Bound<T, U> where Bound<T, U>: AppearsOnTable<QS> {}

impl<T, U, QS> AppearsOnTable<QS> for Bound<T, U> where Bound<T, U>: Expression {}

impl<T, U> NonAggregate for Bound<T, U> {}
