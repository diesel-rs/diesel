use crate::expression::operators::Concat;
use crate::mysql::Mysql;
use crate::query_builder::insert_statement::DefaultValues;
use crate::query_builder::locking_clause::{ForShare, ForUpdate, NoModifier, NoWait, SkipLocked};
use crate::query_builder::{AstPass, QueryFragment};
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

impl<L, R> QueryFragment<Mysql, crate::mysql::backend::MysqlConcatClause> for Concat<L, R>
where
    L: QueryFragment<Mysql>,
    R: QueryFragment<Mysql>,
{
    fn walk_ast<'b>(
        &'b self,
        mut out: crate::query_builder::AstPass<'_, 'b, Mysql>,
    ) -> crate::result::QueryResult<()> {
        out.push_sql("CONCAT(");
        self.left.walk_ast(out.reborrow())?;
        out.push_sql(",");
        self.right.walk_ast(out.reborrow())?;
        out.push_sql(")");
        Ok(())
    }
}

