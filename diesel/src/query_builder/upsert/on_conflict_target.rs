use crate::backend::{Backend, SupportsOnConflictClause};
use crate::expression::SqlLiteral;
use crate::query_builder::*;
use crate::query_source::Column;
use crate::result::QueryResult;

#[doc(hidden)]
pub trait OnConflictTarget<Table> {}

#[doc(hidden)]
#[derive(Debug, Clone, Copy)]
pub struct NoConflictTarget;

impl<DB> QueryFragment<DB> for NoConflictTarget
where
    DB: Backend + SupportsOnConflictClause,
{
    fn walk_ast(&self, _: AstPass<DB>) -> QueryResult<()> {
        Ok(())
    }
}

impl<Table> OnConflictTarget<Table> for NoConflictTarget {}

#[doc(hidden)]
#[derive(Debug, Clone, Copy)]
pub struct ConflictTarget<T>(pub T);

impl<DB, T> QueryFragment<DB> for ConflictTarget<T>
where
    DB: Backend + SupportsOnConflictClause,
    T: Column,
{
    fn walk_ast(&self, mut out: AstPass<DB>) -> QueryResult<()> {
        out.push_sql(" (");
        out.push_identifier(T::NAME)?;
        out.push_sql(")");
        Ok(())
    }
}

impl<T> OnConflictTarget<T::Table> for ConflictTarget<T> where T: Column {}

impl<DB, ST> QueryFragment<DB> for ConflictTarget<SqlLiteral<ST>>
where
    DB: Backend + SupportsOnConflictClause,
    SqlLiteral<ST>: QueryFragment<DB>,
{
    fn walk_ast(&self, mut out: AstPass<DB>) -> QueryResult<()> {
        out.push_sql(" ");
        self.0.walk_ast(out.reborrow())?;
        Ok(())
    }
}

impl<Tab, ST> OnConflictTarget<Tab> for ConflictTarget<SqlLiteral<ST>> {}

impl<DB, T> QueryFragment<DB> for ConflictTarget<(T,)>
where
    DB: Backend + SupportsOnConflictClause,
    T: Column,
{
    fn walk_ast(&self, mut out: AstPass<DB>) -> QueryResult<()> {
        out.push_sql(" (");
        out.push_identifier(T::NAME)?;
        out.push_sql(")");
        Ok(())
    }
}

impl<T> OnConflictTarget<T::Table> for ConflictTarget<(T,)> where T: Column {}

#[diesel_derives::__diesel_for_each_tuple]
impl<_DB, T, #[repeat] O> QueryFragment<_DB> for ConflictTarget<(T, O)>
where
    _DB: Backend + SupportsOnConflictClause,
    T: Column,
    O: Column<Table = T::Table>,
{
    fn walk_ast(&self, mut out: AstPass<_DB>) -> QueryResult<()> {
        out.push_sql(" (");
        out.push_identifier(T::NAME)?;
        #[repeat]
        {
            out.push_sql(", ");
            out.push_identifier(O::NAME)?;
        }
        out.push_sql(")");
        Ok(())
    }
}

#[diesel_derives::__diesel_for_each_tuple]
impl<T, #[repeat] O> OnConflictTarget<T::Table> for ConflictTarget<(T, O)>
where
    T: Column,
    O: Column<Table = T::Table>,
{
}
