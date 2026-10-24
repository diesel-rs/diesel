//! The SQLite query builder

use super::backend::Sqlite;
use crate::query_builder::QueryBuilder;
use crate::result::QueryResult;
use alloc::string::String;
use zeroize::Zeroize;

mod batch_update;
mod limit_offset;
mod query_fragment_impls;
mod returning;

/// Constructs SQL queries for use with the SQLite backend
#[allow(missing_debug_implementations)]
#[derive(Default)]
pub struct SqliteQueryBuilder {
    sql: String,
}

impl SqliteQueryBuilder {
    /// Construct a new query builder with an empty query
    pub fn new() -> Self {
        SqliteQueryBuilder::default()
    }

    pub(crate) fn with_capacity(capacity: usize) -> Self {
        Self {
            sql: String::with_capacity(capacity),
        }
    }

    pub(crate) fn as_str(&self) -> &str {
        &self.sql
    }
}

impl QueryBuilder<Sqlite> for SqliteQueryBuilder {
    fn push_sql(&mut self, sql: &str) {
        self.sql.push_str(sql);
    }

    fn push_identifier(&mut self, identifier: &str) -> QueryResult<()> {
        self.push_sql("`");
        for (index, fragment) in identifier.split('`').enumerate() {
            if index != 0 {
                self.push_sql("``");
            }
            self.push_sql(fragment);
        }
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

impl Zeroize for SqliteQueryBuilder {
    fn zeroize(&mut self) {
        self.sql.zeroize();
    }
}
