use backend::Backend;
use query_builder::*;
use result::QueryResult;
use std::marker::PhantomData;
use super::{Expression, SelectableExpression, NonAggregate};
use types::HasSqlType;

#[derive(Debug, Clone)]
/// Available for when you truly cannot represent something using the expression
/// DSL. You will need to provide the type of the expression, in addition to the
/// SQL. The compiler will be unable to verify the correctness of this type.
pub struct SqlLiteral<ST> {
    sql: String,
    _marker: PhantomData<ST>,
}

impl<ST> SqlLiteral<ST> {
    pub fn new(sql: String) -> Self {
        SqlLiteral {
            sql: sql,
            _marker: PhantomData,
        }
    }
}

impl<ST> Expression for SqlLiteral<ST> {
    type SqlType = ST;
}

impl<ST, DB> QueryFragment<DB> for SqlLiteral<ST> where
    DB: Backend + HasSqlType<ST>,
{
    fn to_sql(&self, out: &mut DB::QueryBuilder) -> BuildQueryResult {
        out.push_sql(&self.sql);
        Ok(())
    }

    fn collect_binds(&self, _out: &mut DB::BindCollector) -> QueryResult<()> {
        Ok(())
    }
}

impl<ST> Query for SqlLiteral<ST> {
    type SqlType = ST;
}

impl<QS, ST> SelectableExpression<QS> for SqlLiteral<ST> {
}

impl<ST> NonAggregate for SqlLiteral<ST> {
}

pub fn sql<ST>(sql: &str) -> SqlLiteral<ST> {
    SqlLiteral::new(sql.into())
}
