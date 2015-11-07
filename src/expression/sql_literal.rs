use query_builder::{QueryBuilder, BuildQueryResult};
use std::marker::PhantomData;
use super::{Expression, SelectableExpression};
use types::NativeSqlType;

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

impl<ST: NativeSqlType> Expression for SqlLiteral<ST> {
    type SqlType = ST;

    fn to_sql<B: QueryBuilder>(&self, out: &mut B) -> BuildQueryResult {
        out.push_sql(&self.sql);
        Ok(())
    }
}

impl<QS, ST: NativeSqlType> SelectableExpression<QS> for SqlLiteral<ST> {
}
