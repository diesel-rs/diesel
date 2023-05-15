use crate::expression::array_comparison::{In, Many, MaybeEmpty, NotIn};
use crate::pg::backend::PgStyleArrayComparison;
use crate::pg::types::sql_types::Array;
use crate::pg::Pg;
use crate::query_builder::locking_clause::{
    ForKeyShare, ForNoKeyUpdate, ForShare, ForUpdate, NoModifier, NoWait, SkipLocked,
};
use crate::query_builder::upsert::into_conflict_clause::OnConflictSelectWrapper;
use crate::query_builder::upsert::on_conflict_target_decorations::DecoratedConflictTarget;
use crate::query_builder::{AstPass, QueryFragment};
use crate::result::QueryResult;
use crate::serialize::ToSql;
use crate::sql_types::{HasSqlType, SingleValue};

impl QueryFragment<Pg> for ForUpdate {
    fn walk_ast<'b>(&'b self, mut out: AstPass<'_, 'b, Pg>) -> QueryResult<()> {
        out.push_sql(" FOR UPDATE");
        Ok(())
    }
}

impl QueryFragment<Pg> for ForNoKeyUpdate {
    fn walk_ast<'b>(&'b self, mut out: AstPass<'_, 'b, Pg>) -> QueryResult<()> {
        out.push_sql(" FOR NO KEY UPDATE");
        Ok(())
    }
}

impl QueryFragment<Pg> for ForShare {
    fn walk_ast<'b>(&'b self, mut out: AstPass<'_, 'b, Pg>) -> QueryResult<()> {
        out.push_sql(" FOR SHARE");
        Ok(())
    }
}

impl QueryFragment<Pg> for ForKeyShare {
    fn walk_ast<'b>(&'b self, mut out: AstPass<'_, 'b, Pg>) -> QueryResult<()> {
        out.push_sql(" FOR KEY SHARE");
        Ok(())
    }
}

impl QueryFragment<Pg> for NoModifier {
    fn walk_ast<'b>(&'b self, _out: AstPass<'_, 'b, Pg>) -> QueryResult<()> {
        Ok(())
    }
}

impl QueryFragment<Pg> for SkipLocked {
    fn walk_ast<'b>(&'b self, mut out: AstPass<'_, 'b, Pg>) -> QueryResult<()> {
        out.push_sql(" SKIP LOCKED");
        Ok(())
    }
}

impl QueryFragment<Pg> for NoWait {
    fn walk_ast<'b>(&'b self, mut out: AstPass<'_, 'b, Pg>) -> QueryResult<()> {
        out.push_sql(" NOWAIT");
        Ok(())
    }
}

impl<T, U> QueryFragment<Pg, PgStyleArrayComparison> for In<T, U>
where
    T: QueryFragment<Pg>,
    U: QueryFragment<Pg> + MaybeEmpty,
{
    fn walk_ast<'b>(&'b self, mut out: AstPass<'_, 'b, Pg>) -> QueryResult<()> {
        self.left.walk_ast(out.reborrow())?;
        out.push_sql(" = ANY(");
        self.values.walk_ast(out.reborrow())?;
        out.push_sql(")");
        Ok(())
    }
}

impl<T, U> QueryFragment<Pg, PgStyleArrayComparison> for NotIn<T, U>
where
    T: QueryFragment<Pg>,
    U: QueryFragment<Pg> + MaybeEmpty,
{
    fn walk_ast<'b>(&'b self, mut out: AstPass<'_, 'b, Pg>) -> QueryResult<()> {
        self.left.walk_ast(out.reborrow())?;
        out.push_sql(" != ALL(");
        self.values.walk_ast(out.reborrow())?;
        out.push_sql(")");
        Ok(())
    }
}

impl<ST, I> QueryFragment<Pg, PgStyleArrayComparison> for Many<ST, I>
where
    ST: SingleValue,
    Vec<I>: ToSql<Array<ST>, Pg>,
    Pg: HasSqlType<ST>,
{
    fn walk_ast<'b>(&'b self, mut out: AstPass<'_, 'b, Pg>) -> QueryResult<()> {
        out.push_bind_param::<Array<ST>, Vec<I>>(&self.values)
    }
}

impl<T, U> QueryFragment<Pg, crate::pg::backend::PgOnConflictClause>
    for DecoratedConflictTarget<T, U>
where
    T: QueryFragment<Pg>,
    U: QueryFragment<Pg>,
{
    fn walk_ast<'b>(&'b self, mut out: AstPass<'_, 'b, Pg>) -> QueryResult<()> {
        self.target.walk_ast(out.reborrow())?;
        self.where_clause.walk_ast(out.reborrow())?;
        Ok(())
    }
}

impl<S> QueryFragment<crate::pg::Pg> for OnConflictSelectWrapper<S>
where
    S: QueryFragment<crate::pg::Pg>,
{
    fn walk_ast<'b>(&'b self, out: AstPass<'_, 'b, crate::pg::Pg>) -> QueryResult<()> {
        self.0.walk_ast(out)
    }
}
