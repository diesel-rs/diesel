use std::marker::PhantomData;

use crate::backend::sql_dialect::on_conflict_clause;
use crate::expression::{AppearsOnTable, Expression};
use crate::query_builder::*;
use crate::query_source::*;

#[doc(hidden)]
#[derive(Debug, Clone, Copy, QueryId)]
pub struct DoNothing<T>(PhantomData<T>);

impl<T> DoNothing<T> {
    pub(crate) fn new() -> Self {
        Self(PhantomData)
    }
}

impl<T, DB> QueryFragment<DB> for DoNothing<T>
where
    DB: Backend,
    Self: QueryFragment<DB, DB::OnConflictClause>,
{
    fn walk_ast<'b>(&'b self, pass: AstPass<'_, 'b, DB>) -> QueryResult<()> {
        <Self as QueryFragment<DB, DB::OnConflictClause>>::walk_ast(self, pass)
    }
}

impl<DB, T, SD> QueryFragment<DB, SD> for DoNothing<T>
where
    DB: Backend<OnConflictClause = SD>,
    SD: on_conflict_clause::PgLikeOnConflictClause,
{
    fn walk_ast<'b>(&'b self, mut out: AstPass<'_, 'b, DB>) -> QueryResult<()> {
        out.push_sql(" DO NOTHING");
        Ok(())
    }
}

#[doc(hidden)]
#[derive(Debug, Clone, Copy, QueryId)]
pub struct DoUpdate<T, Tab> {
    pub(crate) changeset: T,
    tab: PhantomData<Tab>,
}

impl<T, Tab> DoUpdate<T, Tab> {
    pub(crate) fn new(changeset: T) -> Self {
        DoUpdate {
            changeset,
            tab: PhantomData,
        }
    }
}

impl<DB, T, Tab> QueryFragment<DB> for DoUpdate<T, Tab>
where
    DB: Backend,
    Self: QueryFragment<DB, DB::OnConflictClause>,
{
    fn walk_ast<'b>(&'b self, pass: AstPass<'_, 'b, DB>) -> QueryResult<()> {
        <Self as QueryFragment<DB, DB::OnConflictClause>>::walk_ast(self, pass)
    }
}

impl<DB, T, Tab, SD> QueryFragment<DB, SD> for DoUpdate<T, Tab>
where
    DB: Backend<OnConflictClause = SD>,
    T: QueryFragment<DB>,
    SD: on_conflict_clause::PgLikeOnConflictClause,
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

impl<DB, T, SD> QueryFragment<DB, SD> for Excluded<T>
where
    DB: Backend<OnConflictClause = SD>,
    T: Column,
    SD: on_conflict_clause::PgLikeOnConflictClause,
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
