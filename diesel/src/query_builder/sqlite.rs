use backend::{Sqlite, SqliteType};
use super::{QueryBuilder, BuildQueryResult, Context};
use types::HasSqlType;

#[doc(hidden)]
pub struct SqliteQueryBuilder {
    pub sql: String,
    pub bind_params: Vec<(SqliteType, Option<Vec<u8>>)>,
}

impl SqliteQueryBuilder {
    pub fn new() -> Self {
        SqliteQueryBuilder {
            sql: String::new(),
            bind_params: Vec::new(),
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

    fn push_bound_value<T>(&mut self, bind: Option<Vec<u8>>) where
        Sqlite: HasSqlType<T>,
    {
        self.push_sql("?");
        self.bind_params.push((Sqlite::metadata(), bind));
    }

    fn push_context(&mut self, _context: Context) {
    }

    fn pop_context(&mut self) {
    }
}
