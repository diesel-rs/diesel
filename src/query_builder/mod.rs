pub mod pg;

use std::error::Error;

pub type Binds = Vec<Option<Vec<u8>>>;
pub type BuildQueryResult = Result<(), Box<Error>>;

pub trait QueryBuilder {
    fn push_sql(&mut self, sql: &str);
    fn push_identifier(&mut self, identifier: &str) -> BuildQueryResult;
    fn push_bound_value(&mut self, binds: Option<Vec<u8>>);
}
