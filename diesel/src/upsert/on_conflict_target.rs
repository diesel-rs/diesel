use crate::backend::{Backend, SupportsOnConflictClause};
use crate::expression::SqlLiteral;
use crate::query_builder::*;
use crate::query_source::Column;
use crate::result::QueryResult;

#[cfg(feature = "postgres")]
pub use pg::on_constraint;

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

impl<T> OnConflictTarget<T::Table> for ConflictTarget<T>
where
    T: Column,
{
}

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

macro_rules! on_conflict_tuples {
    ($($col:ident),+) => {
        impl<DB, T, $($col),+> QueryFragment<DB> for ConflictTarget<(T, $($col),+)> where
            DB: Backend + SupportsOnConflictClause,
            T: Column,
            $($col: Column<Table=T::Table>,)+
        {
            fn walk_ast(&self, mut out: AstPass<DB>) -> QueryResult<()> {
                out.push_sql(" (");
                out.push_identifier(T::NAME)?;
                $(
                    out.push_sql(", ");
                    out.push_identifier($col::NAME)?;
                )+
                out.push_sql(")");
                Ok(())
            }
        }

        impl<T, $($col),+> OnConflictTarget<T::Table> for ConflictTarget<(T, $($col),+)> where
            T: Column,
            $($col: Column<Table=T::Table>,)+
        {
        }
    }
}

on_conflict_tuples!(U);
on_conflict_tuples!(U, V);
on_conflict_tuples!(U, V, W);
on_conflict_tuples!(U, V, W, X);
on_conflict_tuples!(U, V, W, X, Y);
on_conflict_tuples!(U, V, W, X, Y, Z);
