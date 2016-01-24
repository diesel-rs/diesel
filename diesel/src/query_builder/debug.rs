use backend::Debug;
use super::{QueryBuilder, BuildQueryResult, Context};
use types::HasSqlType;

#[doc(hidden)]
pub struct DebugQueryBuilder {
    pub sql: String,
    pub bind_types: Vec<u32>,
    context_stack: Vec<Context>,
}

impl DebugQueryBuilder {
    pub fn new() -> Self {
        DebugQueryBuilder {
            sql: String::new(),
            bind_types: Vec::new(),
            context_stack: Vec::new(),
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

    fn push_bound_value<T>(&mut self, bind: Option<Vec<u8>>) where
        Debug: HasSqlType<T>,
    {
        match (self.context_stack.first(), bind) {
            (Some(&Context::Insert), None) => self.push_sql("DEFAULT"),
            _ => self.push_sql("?"),
        }
    }

    fn push_context(&mut self, context: Context) {
        self.context_stack.push(context);
    }

    fn pop_context(&mut self) {
        self.context_stack.pop();
    }
}
