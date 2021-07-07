use std::marker::PhantomData;

use super::*;
use crate::backend::Backend;
use crate::query_builder::*;
use crate::result::QueryResult;
use crate::serialize::ToSql;
use crate::sql_types::{DieselNumericOps, HasSqlType, SqlType};

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
    fn walk_ast(&self, mut pass: AstPass<DB>) -> QueryResult<()> {
        pass.push_bind_param(&self.item)?;
        Ok(())
    }

    fn walk_ast_primary_key(&self, _primary_key:String, mut pass: AstPass<DB>) -> QueryResult<()> {
        use crate::result::Error::SerializationError;
        use crate::serialize::Output;        
        // use crate::sql_types::TypeMetadata;
        
        let mut to_sql_output = Output::new1(Vec::new());
        let _is_null = self.item
            .to_sql(&mut to_sql_output)
            .map_err(SerializationError)?;
        let bytes = to_sql_output.into_inner();                      
        pass.push_sql(std::str::from_utf8(&bytes).unwrap());

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
