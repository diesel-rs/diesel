use super::backend::Sqlite;
use query_builder::{QueryBuilder, BuildQueryResult};

pub mod functions;
#[doc(hidden)]
pub mod nodes;

#[allow(missing_debug_implementations)]
pub struct SqliteQueryBuilder {
    pub sql: String,
}

impl SqliteQueryBuilder {
    pub fn new() -> Self {
        SqliteQueryBuilder {
            sql: String::new(),
        }
    }
}

impl QueryBuilder<Sqlite> for SqliteQueryBuilder {
    fn push_sql(&mut self, sql: &str) {
        self.sql.push_str(sql);
    }

    fn push_identifier(&mut self, identifier: &str) -> BuildQueryResult {
        self.push_sql("`");
        self.push_sql(&identifier.replace("`", "``"));
        self.push_sql("`");
        Ok(())
    }

    fn push_bind_param(&mut self) {
        self.push_sql("?");
    }
}
