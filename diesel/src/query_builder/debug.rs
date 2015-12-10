use super::{QueryBuilder, BuildQueryResult};
use types::NativeSqlType;

#[doc(hidden)]
pub struct DebugQueryBuilder {
    pub sql: String,
    pub bind_types: Vec<u32>,
    bind_idx: u32,
}

impl DebugQueryBuilder {
    pub fn new() -> Self {
        DebugQueryBuilder {
            sql: String::new(),
            bind_types: Vec::new(),
            bind_idx: 0,
        }
    }
}

impl QueryBuilder for DebugQueryBuilder {
    fn push_sql(&mut self, sql: &str) {
        self.sql.push_str(sql);
    }

    fn push_identifier(&mut self, identifier: &str) -> BuildQueryResult {
        Ok(self.push_sql(&identifier))
    }

    fn push_bound_value(&mut self, _tpe: &NativeSqlType, _bind: Option<Vec<u8>>) {
        self.bind_idx += 1;
        let sql = format!("${}", self.bind_idx);
        self.push_sql(&sql);
    }
}
