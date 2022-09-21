use crate::backend::{sql_dialect, Backend};
use crate::expression::{AppearsOnTable, Expression};
use crate::query_builder::where_clause::{WhereClause, NoWhereClause};
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

impl<DB, T> QueryFragment<DB, T> for DoNothing
where
    DB: Backend,
    T: sql_dialect::on_conflict_clause::SupportsOnConflictClause,
{
    fn walk_ast<'b>(&'b self, mut out: AstPass<'_, 'b, DB>) -> QueryResult<()> {
        out.push_sql(" DO NOTHING");
        Ok(())
    }
}

#[doc(hidden)]
#[derive(Debug, Clone, Copy, QueryId)]
pub struct DoUpdate<T, WhereClause> {
    changeset: T,
    where_clause: WhereClause,
}

impl<T, P> DoUpdate<T, P>
where
    P: Expression,
{
    pub(crate) fn new(changeset: T, predicate: P) -> Self {
        DoUpdate {
            changeset,
            where_clause: NoWhereClause.and(predicate),
        }
    }
}

impl<DB, T, U> QueryFragment<DB> for DoUpdate<T, U>
where
    DB: Backend,
    Self: QueryFragment<DB, DB::OnConflictClause>,
{
    fn walk_ast<'b>(&'b self, pass: AstPass<'_, 'b, DB>) -> QueryResult<()> {
        <Self as QueryFragment<DB, DB::OnConflictClause>>::walk_ast(self, pass)
    }
}

impl<DB, T, SP, U> QueryFragment<DB, SP> for DoUpdate<T, U>
where
    DB: Backend,
    SP: sql_dialect::on_conflict_clause::SupportsOnConflictClause,
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

impl<DB, T, SP> QueryFragment<DB, SP> for Excluded<T>
where
    DB: Backend,
    SP: sql_dialect::on_conflict_clause::SupportsOnConflictClause,
    T: Column,
{
    fn walk_ast<'b>(&'b self, mut out: AstPass<'_, 'b, DB>) -> QueryResult<()> {
        out.push_sql("excluded.");
        out.push_identifier(T::NAME)?;
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
