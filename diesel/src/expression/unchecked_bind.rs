#![cfg(feature = "with-deprecated")]

use std::marker::PhantomData;

use backend::Backend;
use query_builder::*;
use result::QueryResult;
use types::{HasSqlType, ToSql};

#[derive(Debug, Clone, Copy)]
#[must_use = "Queries are only executed when calling `load`, `get_result` or similar."]
pub struct UncheckedBind<Query, Value, ST> {
    query: Query,
    value: Value,
    _marker: PhantomData<ST>,
}

impl<Query, Value, ST> UncheckedBind<Query, Value, ST> {
    pub fn new(query: Query, value: Value) -> Self {
        UncheckedBind {
            query,
            value,
            _marker: PhantomData,
        }
    }

    pub fn bind<ST2, Value2>(self, value: Value2) -> UncheckedBind<Self, Value2, ST2> {
        UncheckedBind::new(self, value)
    }
}

impl<Query, Value, ST> QueryId for UncheckedBind<Query, Value, ST>
where
    Query: QueryId,
    ST: QueryId,
{
    type QueryId = UncheckedBind<Query::QueryId, (), ST::QueryId>;

    const HAS_STATIC_QUERY_ID: bool = Query::HAS_STATIC_QUERY_ID && ST::HAS_STATIC_QUERY_ID;
}

impl<Query, Value, ST, DB> QueryFragment<DB> for UncheckedBind<Query, Value, ST>
where
    DB: Backend + HasSqlType<ST>,
    Query: QueryFragment<DB>,
    Value: ToSql<ST, DB>,
{
    fn walk_ast(&self, mut out: AstPass<DB>) -> QueryResult<()> {
        self.query.walk_ast(out.reborrow())?;
        out.push_bind_param_value_only(&self.value)?;
        Ok(())
    }
}

impl<Q, Value, ST> Query for UncheckedBind<Q, Value, ST>
where
    Q: Query,
{
    type SqlType = Q::SqlType;
}
