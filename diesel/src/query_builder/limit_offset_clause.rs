use super::{AstPass, QueryFragment};
use crate::backend::{Backend, SupportsLonelyOffset};
use crate::query_builder::QueryId;
use crate::result::QueryResult;

#[doc(hidden)]
#[derive(Debug, Clone, Copy, QueryId)]
pub struct LimitOffsetClause<Limit, Offset> {
    pub(crate) limit_clause: Limit,
    pub(crate) offset_clause: Offset,
}

#[allow(missing_debug_implementations)]
pub struct BoxedLimitOffsetClause<'a, DB> {
    pub(crate) limit: Option<Box<dyn QueryFragment<DB> + Send + 'a>>,
    pub(crate) offset: Option<Box<dyn QueryFragment<DB> + Send + 'a>>,
}

impl<'a, DB, L, O> From<LimitOffsetClause<L, O>> for BoxedLimitOffsetClause<'a, DB>
where
    DB: Backend + SupportsLonelyOffset,
    L: QueryFragment<DB> + Send + 'a,
    O: QueryFragment<DB> + Send + 'a,
{
    fn from(limit_offset: LimitOffsetClause<L, O>) -> Self {
        Self {
            limit: Some(Box::new(limit_offset.limit_clause)),
            offset: Some(Box::new(limit_offset.offset_clause)),
        }
    }
}

impl<'a, DB> QueryFragment<DB> for BoxedLimitOffsetClause<'a, DB>
where
    DB: Backend + SupportsLonelyOffset,
{
    fn walk_ast(&self, mut out: AstPass<DB>) -> QueryResult<()> {
        if let Some(ref limit) = self.limit {
            limit.walk_ast(out.reborrow())?;
        }
        if let Some(ref offset) = self.offset {
            offset.walk_ast(out.reborrow())?;
        }
        Ok(())
    }
}

impl<L, O, DB> QueryFragment<DB> for LimitOffsetClause<L, O>
where
    L: QueryFragment<DB>,
    O: QueryFragment<DB>,
    DB: Backend + crate::backend::SupportsLonelyOffset,
{
    fn walk_ast(&self, mut out: AstPass<DB>) -> QueryResult<()> {
        self.limit_clause.walk_ast(out.reborrow())?;
        self.offset_clause.walk_ast(out.reborrow())?;
        Ok(())
    }
}
