use std::marker::PhantomData;

use crate::backend::Backend;
use crate::query_builder::QueryBuilder;
use crate::result::QueryResult;

#[doc(inline)]
pub use self::query_fragment_impls::DuplicatedKeys;

mod limit_offset;
mod query_fragment_impls;


/// The MySQL-Like query builder
#[allow(missing_debug_implementations)]
pub struct MysqlLikeQueryBuilder<B: Backend> {
    sql: String,
    _phantom: PhantomData<B>,
}

impl<B: Backend> Default for MysqlLikeQueryBuilder<B> {
    fn default() -> Self {
        Self {
            sql: String::default(),
            _phantom: PhantomData,
        }
    }
}

impl<B: Backend> MysqlLikeQueryBuilder<B> {
    /// Constructs a new query builder with an empty query
    pub fn new() -> Self {
        MysqlLikeQueryBuilder::default()
    }
}

impl<B: Backend> QueryBuilder<B> for MysqlLikeQueryBuilder<B> {
    fn push_sql(&mut self, sql: &str) {
        self.sql.push_str(sql);
    }

    fn push_identifier(&mut self, identifier: &str) -> QueryResult<()> {
        self.push_sql("`");
        self.push_sql(&identifier.replace('`', "``"));
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
