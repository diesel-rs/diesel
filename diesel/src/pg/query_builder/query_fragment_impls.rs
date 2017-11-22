use pg::Pg;
use query_builder::{AstPass, QueryFragment};
use query_builder::for_update_clause::ForUpdateClause;
use result::QueryResult;

impl QueryFragment<Pg> for ForUpdateClause {
    fn walk_ast(&self, mut out: AstPass<Pg>) -> QueryResult<()> {
        out.push_sql(" FOR UPDATE");
        Ok(())
    }
}
