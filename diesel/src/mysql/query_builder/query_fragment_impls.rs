use mysql::Mysql;
use query_builder::{AstPass, QueryFragment};
use query_builder::for_update_clause::ForUpdateClause;
use result::QueryResult;

impl QueryFragment<Mysql> for ForUpdateClause {
    fn walk_ast(&self, mut out: AstPass<Mysql>) -> QueryResult<()> {
        out.push_sql(" FOR UPDATE");
        Ok(())
    }
}
