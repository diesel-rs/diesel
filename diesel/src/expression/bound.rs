use std::marker::PhantomData;

use super::*;
use crate::query_builder::*;
use crate::result::QueryResult;
use crate::serialize::ToSql;
use crate::sql_types::DieselNumericOps;

#[doc(hidden)] // This is used by the `AsExpression` derive
#[derive(Debug, Clone, Copy, DieselNumericOps)]
pub struct Bound<T, U> {
    pub(crate) item: U,
    _marker: PhantomData<T>,
}

impl<T, U> Bound<T, U> {
    #[doc(hidden)] // This is used by the `AsExpression` derive
    pub fn new(item: U) -> Self {
        Bound {
            item,
            _marker: PhantomData,
        }
    }
}

impl<T, U> Expression for Bound<T, U>
where
    T: SqlType + TypedExpressionType,
{
    type SqlType = T;
}

impl<T, U, DB> QueryFragment<DB> for Bound<T, U>
where
    DB: Backend + HasSqlType<T>,
    U: ToSql<T, DB>,
{
    fn walk_ast<'b>(&'b self, mut pass: AstPass<'_, 'b, DB>) -> QueryResult<()> {
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

impl<T, U, GB> ValidGrouping<GB> for Bound<T, U> {
    type IsAggregate = is_aggregate::Never;
}
