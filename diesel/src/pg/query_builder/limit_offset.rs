use crate::pg::Pg;
use crate::query_builder::limit_offset_clause::{BoxedLimitOffsetClause, LimitOffsetClause};
use crate::query_builder::{AstPass, IntoBoxedClause, QueryFragment};
use crate::result::QueryResult;

impl<'a, L, O> IntoBoxedClause<'a, Pg> for LimitOffsetClause<L, O>
where
    L: QueryFragment<Pg> + Send + 'a,
    O: QueryFragment<Pg> + Send + 'a,
{
    type BoxedClause = BoxedLimitOffsetClause<'a, Pg>;

    fn into_boxed(self) -> Self::BoxedClause {
        BoxedLimitOffsetClause {
            limit: Some(Box::new(self.limit_clause)),
            offset: Some(Box::new(self.offset_clause)),
        }
    }
}

impl<'a> QueryFragment<Pg> for BoxedLimitOffsetClause<'a, Pg> {
    fn walk_ast<'b>(&'b self, mut out: AstPass<'_, 'b, Pg>) -> QueryResult<()> {
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
    fn walk_ast<'b>(&'b self, mut out: AstPass<'_, 'b, Pg>) -> QueryResult<()> {
        self.limit_clause.walk_ast(out.reborrow())?;
        self.offset_clause.walk_ast(out.reborrow())?;
        Ok(())
    }
}
