use crate::backend::Backend;
use crate::query_builder::*;
use crate::query_source::Column;
use crate::result::QueryResult;

/// Represents the column list for use in an insert statement.
///
/// This trait is implemented by columns and tuples of columns.
pub trait ColumnList {
    /// The table these columns belong to
    type Table;

    /// Generate the SQL for this column list.
    ///
    /// Column names must *not* be qualified.
    fn walk_ast<DB: Backend>(&self, out: AstPass<DB>) -> QueryResult<()>;
}

impl<C> ColumnList for C
where
    C: Column,
{
    type Table = <C as Column>::Table;

    fn walk_ast<DB: Backend>(&self, mut out: AstPass<DB>) -> QueryResult<()> {
        out.push_identifier(C::NAME)?;
        Ok(())
    }
}
