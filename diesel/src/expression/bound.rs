use std::marker::PhantomData;

use backend::Backend;
use query_builder::*;
use result::QueryResult;
use super::*;
use types::{HasSqlType, ToSql};

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

    fn walk_ast(&self, pass: AstPass<DB>) -> QueryResult<()> {
        if let AstPass::CollectBinds(out) = pass {
            out.push_bound_value(&self.item)?;
        }
        Ok(())
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
}

impl<T, U, QS> AppearsOnTable<QS> for Bound<T, U> where
    Bound<T, U>: Expression,
{
}

impl<T, U> NonAggregate for Bound<T, U> where
    Bound<T, U>: Expression,
{
}
