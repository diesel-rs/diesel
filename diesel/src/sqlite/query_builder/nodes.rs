use query_builder::*;
use result::QueryResult;
use sqlite::Sqlite;

#[derive(Debug, Copy, Clone)]
pub struct Replace;

impl QueryFragment<Sqlite> for Replace {
    fn walk_ast(&self, mut out: AstPass<Sqlite>) -> QueryResult<()> {
        out.push_sql("REPLACE");
        Ok(())
    }
}

impl_query_id!(Replace);
