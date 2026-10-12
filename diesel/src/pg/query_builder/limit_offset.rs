use crate::pg::Pg;
use crate::query_builder::limit_offset_clause::{
    BoxedCloneLimitOffsetClause, BoxedLimitOffsetClause, LimitOffsetClause,
};
use crate::query_builder::{AstPass, IntoBoxedClause, IntoBoxedCloneClause, QueryFragment};
use crate::result::QueryResult;
use alloc::sync::Arc;

impl<'a, L, O> IntoBoxedCloneClause<'a, Pg> for LimitOffsetClause<L, O>
where
    L: QueryFragment<Pg> + Send + Sync + 'a,
    O: QueryFragment<Pg> + Send + Sync + 'a,
{
    type BoxedCloneClause = BoxedCloneLimitOffsetClause<'a, Pg>;

    fn into_boxed_clone(self) -> Self::BoxedCloneClause {
        BoxedCloneLimitOffsetClause {
            limit: Some(Arc::new(self.limit_clause)),
            offset: Some(Arc::new(self.offset_clause)),
        }
    }
}

/// Walk the limit and the offset of a boxed limit/offset clause, whichever pointer holds them.
fn walk_boxed_limit_offset<'b, P>(
    limit: Option<&'b P>,
    offset: Option<&'b P>,
    mut out: AstPass<'_, 'b, Pg>,
) -> QueryResult<()>
where
    P: QueryFragment<Pg>,
{
    if let Some(limit) = limit {
        limit.walk_ast(out.reborrow())?;
    }
    if let Some(offset) = offset {
        offset.walk_ast(out.reborrow())?;
    }
    Ok(())
}

impl QueryFragment<Pg> for BoxedCloneLimitOffsetClause<'_, Pg> {
    fn walk_ast<'b>(&'b self, out: AstPass<'_, 'b, Pg>) -> QueryResult<()> {
        walk_boxed_limit_offset(self.limit.as_ref(), self.offset.as_ref(), out)
    }
}

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

impl QueryFragment<Pg> for BoxedLimitOffsetClause<'_, Pg> {
    fn walk_ast<'b>(&'b self, out: AstPass<'_, 'b, Pg>) -> QueryResult<()> {
        walk_boxed_limit_offset(self.limit.as_ref(), self.offset.as_ref(), out)
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
