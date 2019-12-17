use crate::pg::Pg;
use crate::query_builder::locking_clause::{
    ForKeyShare, ForNoKeyUpdate, ForShare, ForUpdate, NoModifier, NoWait, SkipLocked,
};
use crate::query_builder::{AstPass, QueryFragment};
use crate::result::QueryResult;

impl QueryFragment<Pg> for ForUpdate {
    fn walk_ast(&self, mut out: AstPass<Pg>) -> QueryResult<()> {
        out.push_sql(" FOR UPDATE");
        Ok(())
    }
}

impl QueryFragment<Pg> for ForNoKeyUpdate {
    fn walk_ast(&self, mut out: AstPass<Pg>) -> QueryResult<()> {
        out.push_sql(" FOR NO KEY UPDATE");
        Ok(())
    }
}

impl QueryFragment<Pg> for ForShare {
    fn walk_ast(&self, mut out: AstPass<Pg>) -> QueryResult<()> {
        out.push_sql(" FOR SHARE");
        Ok(())
    }
}

impl QueryFragment<Pg> for ForKeyShare {
    fn walk_ast(&self, mut out: AstPass<Pg>) -> QueryResult<()> {
        out.push_sql(" FOR KEY SHARE");
        Ok(())
    }
}

impl QueryFragment<Pg> for NoModifier {
    fn walk_ast(&self, _out: AstPass<Pg>) -> QueryResult<()> {
        Ok(())
    }
}

impl QueryFragment<Pg> for SkipLocked {
    fn walk_ast(&self, mut out: AstPass<Pg>) -> QueryResult<()> {
        out.push_sql(" SKIP LOCKED");
        Ok(())
    }
}

impl QueryFragment<Pg> for NoWait {
    fn walk_ast(&self, mut out: AstPass<Pg>) -> QueryResult<()> {
        out.push_sql(" NOWAIT");
        Ok(())
    }
}
