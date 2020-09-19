use super::backend::Pg;
use crate::query_builder::QueryBuilder;
use crate::result::QueryResult;

mod distinct_on;
mod limit_offset;
pub(crate) mod on_constraint;
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
        self.sql += "$";
        itoa::fmt(&mut self.sql, self.bind_idx).expect("int formating does not fail");
    }

    fn finish(self) -> String {
        self.sql
    }
}
