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

#[derive(Debug, Clone, Copy)]
pub struct DistinctOnClause<T>(pub(crate) T);

impl<T> QueryFragment<Pg> for DistinctOnClause<T>
where
    T: QueryFragment<Pg>,
{
    fn walk_ast(&self, mut out: AstPass<Pg>) -> QueryResult<()> {
        out.push_sql("DISTINCT ON(");
        self.0.walk_ast(out.reborrow())?;
        out.push_sql(")");
        Ok(())
    }
}

impl_query_id!(DistinctOnClause<T>);
