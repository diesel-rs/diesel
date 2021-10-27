use crate::backend::Backend;
use crate::query_builder::{AstPass, QueryFragment, QueryId};
use crate::result::QueryResult;

#[derive(Debug, Clone, Copy, QueryId)]
pub struct NoLockingClause;

impl<DB: Backend> QueryFragment<DB> for NoLockingClause {
    fn walk_ast<'a, 'b>(&'a self, _: AstPass<'_, 'b, DB>) -> QueryResult<()>
    where
        'a: 'b,
    {
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, QueryId)]
pub struct LockingClause<LockMode = ForUpdate, Modifier = NoModifier> {
    pub(crate) lock_mode: LockMode,
    modifier: Modifier,
}

impl<LockMode, Modifier> LockingClause<LockMode, Modifier> {
    pub(crate) fn new(lock_mode: LockMode, modifier: Modifier) -> Self {
        LockingClause {
            lock_mode,
            modifier,
        }
    }
}

impl<DB: Backend, L: QueryFragment<DB>, M: QueryFragment<DB>> QueryFragment<DB>
    for LockingClause<L, M>
{
    fn walk_ast<'a, 'b>(&'a self, mut out: AstPass<'_, 'b, DB>) -> QueryResult<()>
    where
        'a: 'b,
    {
        self.lock_mode.walk_ast(out.reborrow())?;
        self.modifier.walk_ast(out.reborrow())
    }
}

// `LockMode` parameters
// All the different types of row locks that can be acquired.
#[derive(Debug, Clone, Copy, QueryId)]
pub struct ForUpdate;

#[derive(Debug, Clone, Copy, QueryId)]
pub struct ForNoKeyUpdate;

#[derive(Debug, Clone, Copy, QueryId)]
pub struct ForShare;

#[derive(Debug, Clone, Copy, QueryId)]
pub struct ForKeyShare;

// Modifiers
// To be used in conjunction with a lock mode.
#[derive(Debug, Clone, Copy, QueryId)]
pub struct NoModifier;

#[derive(Debug, Clone, Copy, QueryId)]
pub struct SkipLocked;

#[derive(Debug, Clone, Copy, QueryId)]
pub struct NoWait;
