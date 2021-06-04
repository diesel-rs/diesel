use crate::backend::{Backend, SupportsOnConflictClause};
use crate::expression::{AppearsOnTable, Expression};
use crate::query_builder::*;
use crate::query_source::*;
use crate::result::QueryResult;

#[doc(hidden)]
#[derive(Debug, Clone, Copy, QueryId)]
pub struct DoNothing;

impl<DB> QueryFragment<DB> for DoNothing
where
    DB: Backend + SupportsOnConflictClause,
{
    fn walk_ast(&self, mut out: AstPass<DB>) -> QueryResult<()> {
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
    DB: Backend + SupportsOnConflictClause,
    T: QueryFragment<DB>,
{
    fn walk_ast(&self, mut out: AstPass<DB>) -> QueryResult<()> {
        out.unsafe_to_cache_prepared();
        if self.changeset.is_noop()? {
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
    DB: Backend + SupportsOnConflictClause,
    T: Column,
{
    fn walk_ast(&self, mut out: AstPass<DB>) -> QueryResult<()> {
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
