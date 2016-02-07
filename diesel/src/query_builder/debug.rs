use backend::Debug;
use super::{BuildQueryResult, QueryBuilder};
use types::HasSqlType;

#[doc(hidden)]
pub struct DebugQueryBuilder {
    pub sql: String,
    pub bind_types: Vec<u32>,
}

impl DebugQueryBuilder {
    pub fn new() -> Self {
        DebugQueryBuilder {
            sql: String::new(),
            bind_types: Vec::new(),
        }
    }
}

impl QueryBuilder<Debug> for DebugQueryBuilder {
    fn push_sql(&mut self, sql: &str) {
        self.sql.push_str(sql);
    }

    fn push_identifier(&mut self, identifier: &str) -> BuildQueryResult {
        self.push_sql("`");
        self.push_sql(&identifier);
        self.push_sql("`");
        Ok(())
    }

    #[allow(unused_variables)]
    fn push_bound_value<T>(&mut self, bind: Option<Vec<u8>>)
        where Debug: HasSqlType<T>,
    {
        self.push_sql("?");
    }
}
