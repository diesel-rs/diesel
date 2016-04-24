use backend::Backend;
use query_builder::{QueryFragment, QueryBuilder, BuildQueryResult};
use result::QueryResult;
use sqlite::Sqlite;

#[derive(Debug, Copy, Clone)]
pub struct Replace;

impl QueryFragment<Sqlite> for Replace {
    fn to_sql(&self, out: &mut <Sqlite as Backend>::QueryBuilder) -> BuildQueryResult {
        out.push_sql("REPLACE");
        Ok(())
    }

    fn collect_binds(&self, _out: &mut <Sqlite as Backend>::BindCollector) -> QueryResult<()> {
        Ok(())
    }

    fn is_safe_to_cache_prepared(&self) -> bool {
        true
    }
}

impl_query_id!(Replace);
