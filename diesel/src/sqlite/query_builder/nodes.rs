use backend::Backend;
use query_builder::*;
use result::QueryResult;
use sqlite::Sqlite;

#[derive(Debug, Copy, Clone)]
pub struct Replace;

impl QueryFragment<Sqlite> for Replace {
    fn to_sql(&self, out: &mut <Sqlite as Backend>::QueryBuilder) -> BuildQueryResult {
        out.push_sql("REPLACE");
        Ok(())
    }

    fn walk_ast(&self, _: AstPass<Sqlite>) -> QueryResult<()> {
        Ok(())
    }
}

impl_query_id!(Replace);
