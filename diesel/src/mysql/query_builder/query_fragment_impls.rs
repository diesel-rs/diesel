use crate::mysql::Mysql;
use crate::query_builder::locking_clause::{ForShare, ForUpdate, NoModifier, NoWait, SkipLocked};
use crate::query_builder::{AstPass, DefaultValues, QueryFragment};
use crate::result::QueryResult;

impl QueryFragment<Mysql> for ForUpdate {
    fn walk_ast<'b>(&'b self, mut out: AstPass<'_, 'b, Mysql>) -> QueryResult<()> {
        out.push_sql(" FOR UPDATE");
        Ok(())
    }
}

impl QueryFragment<Mysql> for ForShare {
    fn walk_ast<'b>(&'b self, mut out: AstPass<'_, 'b, Mysql>) -> QueryResult<()> {
        out.push_sql(" FOR SHARE");
        Ok(())
    }
}

impl QueryFragment<Mysql> for NoModifier {
    fn walk_ast<'b>(&'b self, _out: AstPass<'_, 'b, Mysql>) -> QueryResult<()> {
        Ok(())
    }
}

impl QueryFragment<Mysql> for SkipLocked {
    fn walk_ast<'b>(&'b self, mut out: AstPass<'_, 'b, Mysql>) -> QueryResult<()> {
        out.push_sql(" SKIP LOCKED");
        Ok(())
    }
}

impl QueryFragment<Mysql> for NoWait {
    fn walk_ast<'b>(&'b self, mut out: AstPass<'_, 'b, Mysql>) -> QueryResult<()> {
        out.push_sql(" NOWAIT");
        Ok(())
    }
}

impl QueryFragment<Mysql, crate::mysql::backend::MysqlStyleDefaultValueClause> for DefaultValues {
    fn walk_ast<'b>(&'b self, mut out: AstPass<'_, 'b, Mysql>) -> QueryResult<()> {
        out.push_sql("() VALUES ()");
        Ok(())
    }
}
