use pg::Pg;
use query_builder::limit_offset_clause::{BoxedLimitOffsetClause, LimitOffsetClause};
use query_builder::{AstPass, QueryFragment};
use result::QueryResult;

impl<'a, L, O> From<LimitOffsetClause<L, O>> for BoxedLimitOffsetClause<'a, Pg>
where
    L: QueryFragment<Pg> + 'a,
    O: QueryFragment<Pg> + 'a,
{
    fn from(limit_offset: LimitOffsetClause<L, O>) -> Self {
        Self {
            limit: Some(Box::new(limit_offset.limit_clause)),
            offset: Some(Box::new(limit_offset.offset_clause)),
        }
    }
}

impl<'a> QueryFragment<Pg> for BoxedLimitOffsetClause<'a, Pg> {
    fn walk_ast(&self, mut out: AstPass<Pg>) -> QueryResult<()> {
        if let Some(ref limit) = self.limit {
            limit.walk_ast(out.reborrow())?;
        }
        if let Some(ref offset) = self.offset {
            offset.walk_ast(out.reborrow())?;
        }
        Ok(())
    }
}

impl<L, O> QueryFragment<Pg> for LimitOffsetClause<L, O>
where
    L: QueryFragment<Pg>,
    O: QueryFragment<Pg>,
{
    fn walk_ast(&self, mut out: AstPass<Pg>) -> QueryResult<()> {
        self.limit_clause.walk_ast(out.reborrow())?;
        self.offset_clause.walk_ast(out.reborrow())?;
        Ok(())
    }
}
