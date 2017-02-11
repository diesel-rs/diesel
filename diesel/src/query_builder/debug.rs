use backend::Debug;
use super::{QueryBuilder, BuildQueryResult};

#[doc(hidden)]
#[derive(Debug, Default)]
pub struct DebugQueryBuilder {
    sql: String,
}

impl DebugQueryBuilder {
    pub fn new() -> Self {
        DebugQueryBuilder::default()
    }
}

impl QueryBuilder<Debug> for DebugQueryBuilder {
    fn push_sql(&mut self, sql: &str) {
        self.sql.push_str(sql);
    }

    fn push_identifier(&mut self, identifier: &str) -> BuildQueryResult {
        self.push_sql("`");
        self.push_sql(identifier);
        self.push_sql("`");
        Ok(())
    }

    fn push_bind_param(&mut self) {
        self.push_sql("?");
    }

    fn finish(self) -> String {
        self.sql
    }
}
