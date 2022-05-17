use crate::backend::{sql_dialect, Backend};
use crate::expression::{AppearsOnTable, Expression};
use crate::query_builder::*;
use crate::query_source::*;
use crate::result::QueryResult;

#[doc(hidden)]
#[derive(Debug, Clone, Copy, QueryId)]
pub struct DoNothing;

impl<DB> QueryFragment<DB> for DoNothing
where
    DB: Backend,
    Self: QueryFragment<DB, DB::OnConflictClause>,
{
    fn walk_ast<'b>(&'b self, pass: AstPass<'_, 'b, DB>) -> QueryResult<()> {
        <Self as QueryFragment<DB, DB::OnConflictClause>>::walk_ast(self, pass)
    }
}

impl<DB> QueryFragment<DB, sql_dialect::on_conflict_clause::PgLikeOnConflictClause> for DoNothing
where
    DB: Backend<
        OnConflictClause = crate::backend::sql_dialect::on_conflict_clause::PgLikeOnConflictClause,
    >,
{
    fn walk_ast<'b>(&'b self, mut out: AstPass<'_, 'b, DB>) -> QueryResult<()> {
        out.push_sql(" DO NOTHING");
        Ok(())
    }
}

impl<DB> QueryFragment<DB, sql_dialect::on_conflict_clause::MysqlLikeOnConflictClause>
    for DoNothing
where
    DB: Backend<
        OnConflictClause = crate::backend::sql_dialect::on_conflict_clause::MysqlLikeOnConflictClause,
    >,
{
    ///
    /// FIXME: This is super wrong right now and will likely cause a panic; MySQL doesn't support
    /// the "DO NOTHING" syntax; the likely workaround would need to have access to at least one
    /// column from the table being modified in this query:
    /// 
    /// https://stackoverflow.com/questions/4596390/insert-on-duplicate-key-do-nothing
    /// 
    fn walk_ast<'b>(&'b self, mut out: AstPass<'_, 'b, DB>) -> QueryResult<()> {
        out.push_sql(" DO NOTHING");
        Ok(())
    }
}

#[doc(hidden)]
#[derive(Debug, Clone, Copy, QueryId)]
pub struct DoUpdate<T> {
    changeset: T,
}

impl<T> DoUpdate<T> {
    pub(crate) fn new(changeset: T) -> Self {
        DoUpdate { changeset }
    }
}

impl<DB, T> QueryFragment<DB> for DoUpdate<T>
where
    DB: Backend,
    Self: QueryFragment<DB, DB::OnConflictClause>,
{
    fn walk_ast<'b>(&'b self, pass: AstPass<'_, 'b, DB>) -> QueryResult<()> {
        <Self as QueryFragment<DB, DB::OnConflictClause>>::walk_ast(self, pass)
    }
}

impl<DB, T>
    QueryFragment<DB, crate::backend::sql_dialect::on_conflict_clause::PgLikeOnConflictClause>
    for DoUpdate<T>
where
    DB: Backend,
    T: QueryFragment<DB>,
{
    fn walk_ast<'b>(&'b self, mut out: AstPass<'_, 'b, DB>) -> QueryResult<()> {
        out.unsafe_to_cache_prepared();
        if self.changeset.is_noop(out.backend())? {
            out.push_sql(" DO NOTHING");
        } else {
            out.push_sql(" DO UPDATE SET ");
            self.changeset.walk_ast(out.reborrow())?;
        }
        Ok(())
    }
}

impl<DB, T>
    QueryFragment<DB, crate::backend::sql_dialect::on_conflict_clause::MysqlLikeOnConflictClause>
    for DoUpdate<T>
where
    DB: Backend,
    T: QueryFragment<DB>,
{
    fn walk_ast<'b>(&'b self, mut out: AstPass<'_, 'b, DB>) -> QueryResult<()> {
        out.unsafe_to_cache_prepared();
        if self.changeset.is_noop(out.backend())? {
            out.push_sql(" DO NOTHING");
        } else {
            out.push_sql(" UPDATE ");
            self.changeset.walk_ast(out.reborrow())?;
        }
        Ok(())
    }
}

#[doc(hidden)]
#[derive(Debug, Clone, Copy, QueryId)]
pub struct Excluded<T>(T);

impl<T> Excluded<T> {
    pub(crate) fn new(t: T) -> Self {
        Excluded(t)
    }
}

impl<DB, T> QueryFragment<DB> for Excluded<T>
where
    DB: Backend,
    Self: QueryFragment<DB, DB::OnConflictClause>,
{
    fn walk_ast<'b>(&'b self, pass: AstPass<'_, 'b, DB>) -> QueryResult<()> {
        <Self as QueryFragment<DB, DB::OnConflictClause>>::walk_ast(self, pass)
    }
}

impl<DB, T>
    QueryFragment<DB, crate::backend::sql_dialect::on_conflict_clause::PgLikeOnConflictClause>
    for Excluded<T>
where
    DB: Backend,
    T: Column,
{
    fn walk_ast<'b>(&'b self, mut out: AstPass<'_, 'b, DB>) -> QueryResult<()> {
        out.push_sql("excluded.");
        out.push_identifier(T::NAME)?;
        Ok(())
    }
}

impl<DB, T>
    QueryFragment<DB, crate::backend::sql_dialect::on_conflict_clause::MysqlLikeOnConflictClause>
    for Excluded<T>
where
    DB: Backend,
    T: Column,
{
    fn walk_ast<'b>(&'b self, mut out: AstPass<'_, 'b, DB>) -> QueryResult<()> {
        out.push_sql("VALUES(");
        out.push_identifier(T::NAME)?;
        out.push_sql(")");
        Ok(())
    }
}

impl<T> Expression for Excluded<T>
where
    T: Expression,
{
    type SqlType = T::SqlType;
}

impl<T> AppearsOnTable<T::Table> for Excluded<T>
where
    T: Column,
    Excluded<T>: Expression,
{
}
