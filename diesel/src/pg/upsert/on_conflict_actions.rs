use expression::{AppearsOnTable, Expression};
use pg::Pg;
use query_builder::*;
use query_source::*;
use result::QueryResult;

/// Represents `excluded.column` in an `ON CONFLICT DO UPDATE` clause.
pub fn excluded<T>(excluded: T) -> Excluded<T> {
    Excluded(excluded)
}

#[doc(hidden)]
#[derive(Debug, Clone, Copy)]
pub struct DoNothing;

impl QueryFragment<Pg> for DoNothing {
    fn walk_ast(&self, mut out: AstPass<Pg>) -> QueryResult<()> {
        out.push_sql(" DO NOTHING");
        Ok(())
    }
}

#[doc(hidden)]
#[derive(Debug, Clone, Copy)]
pub struct DoUpdate<T> {
    changeset: T,
}

impl<T> DoUpdate<T> {
    pub(crate) fn new(changeset: T) -> Self {
        DoUpdate { changeset }
    }
}

impl<T> QueryFragment<Pg> for DoUpdate<T>
where
    T: QueryFragment<Pg>,
{
    fn walk_ast(&self, mut out: AstPass<Pg>) -> QueryResult<()> {
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
#[derive(Debug, Clone, Copy)]
pub struct Excluded<T>(T);

impl<T> QueryFragment<Pg> for Excluded<T>
where
    T: Column,
{
    fn walk_ast(&self, mut out: AstPass<Pg>) -> QueryResult<()> {
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
