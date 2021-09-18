use crate::backend::{Backend, SupportsOnConflictClause};
use crate::expression::SqlLiteral;
use crate::query_builder::*;
use crate::query_source::Column;
use crate::result::QueryResult;

#[doc(hidden)]
pub trait OnConflictTarget<Table> {}

#[doc(hidden)]
#[derive(Debug, Clone, Copy, QueryId)]
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
#[derive(Debug, Clone, Copy, QueryId)]
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

macro_rules! on_conflict_tuples {
    ($(
        $Tuple:tt {
            $(($idx:tt) -> $T:ident, $ST:ident, $TT:ident,)*
        }
    )+) => {
        $(
            impl<_DB, _T, $($T),*> QueryFragment<_DB> for ConflictTarget<(_T, $($T),*)> where
                _DB: Backend + SupportsOnConflictClause,
                _T: Column,
                $($T: Column<Table=_T::Table>,)*
            {
                fn walk_ast(&self, mut out: AstPass<_DB>) -> QueryResult<()> {
                    out.push_sql(" (");
                    out.push_identifier(_T::NAME)?;
                    $(
                        out.push_sql(", ");
                        out.push_identifier($T::NAME)?;
                    )*
                    out.push_sql(")");
                    Ok(())
                }
            }

            impl<_T, $($T),*> OnConflictTarget<_T::Table> for ConflictTarget<(_T, $($T),*)> where
                _T: Column,
                $($T: Column<Table=_T::Table>,)*
            {
            }
        )*
    }
}

__diesel_for_each_tuple!(on_conflict_tuples);
