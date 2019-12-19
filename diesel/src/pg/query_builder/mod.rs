use super::backend::Pg;
use crate::query_builder::QueryBuilder;
use crate::result::QueryResult;

mod distinct_on;
mod query_fragment_impls;
pub use self::distinct_on::DistinctOnClause;

/// The PostgreSQL query builder
#[allow(missing_debug_implementations)]
#[derive(Default)]
pub struct PgQueryBuilder {
    sql: String,
    bind_idx: u32,
}

impl PgQueryBuilder {
    /// Constructs a new query builder with an empty query
    pub fn new() -> Self {
        PgQueryBuilder::default()
    }
}

impl QueryBuilder<Pg> for PgQueryBuilder {
    fn push_sql(&mut self, sql: &str) {
        self.sql.push_str(sql);
    }

    fn push_identifier(&mut self, identifier: &str) -> QueryResult<()> {
        self.push_sql("\"");
        self.push_sql(&identifier.replace('"', "\"\""));
        self.push_sql("\"");
        Ok(())
    }

    fn push_bind_param(&mut self) {
        self.bind_idx += 1;
        let sql = format!("${}", self.bind_idx);
        self.push_sql(&sql);
    }

    fn finish(self) -> String {
        self.sql
    }
}
