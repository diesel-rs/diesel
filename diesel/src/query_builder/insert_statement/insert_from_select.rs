use crate::expression::{Expression, NonAggregate, SelectableExpression};
use crate::insertable::*;
use crate::query_builder::*;
use crate::query_source::Table;

/// Represents `(Columns) SELECT FROM ...` for use in an `INSERT` statement
#[derive(Debug, Clone, Copy, QueryId)]
pub struct InsertFromSelect<Select, Columns> {
    pub(in crate::query_builder) query: Select,
    pub(in crate::query_builder) columns: Columns,
}

impl<Select, Columns> InsertFromSelect<Select, Columns> {
    /// Construct a new `InsertFromSelect` where the target column list is
    /// `T::AllColumns`.
    pub fn new<T>(query: Select) -> Self
    where
        T: Table<AllColumns = Columns>,
        Columns: SelectableExpression<T> + NonAggregate,
    {
        Self {
            query,
            columns: T::all_columns(),
        }
    }

    /// Replace the target column list
    pub fn with_columns<C>(self, columns: C) -> InsertFromSelect<Select, C> {
        InsertFromSelect {
            query: self.query,
            columns,
        }
    }
}

impl<DB, Select, Columns> CanInsertInSingleQuery<DB> for InsertFromSelect<Select, Columns>
where
    DB: Backend,
{
    fn rows_to_insert(&self) -> Option<usize> {
        None
    }
}

impl<DB, Select, Columns> QueryFragment<DB> for InsertFromSelect<Select, Columns>
where
    DB: Backend,
    Columns: ColumnList + Expression,
    Select: Query<SqlType = Columns::SqlType> + QueryFragment<DB>,
{
    fn walk_ast<'b>(&'b self, mut out: AstPass<'_, 'b, DB>) -> QueryResult<()> {
        out.push_sql("(");
        self.columns.walk_ast(out.reborrow())?;
        out.push_sql(") ");
        self.query.walk_ast(out.reborrow())?;
        Ok(())
    }
}

impl<Select, Columns> UndecoratedInsertRecord<Columns::Table> for InsertFromSelect<Select, Columns>
where
    Columns: ColumnList + Expression,
    Select: Query<SqlType = Columns::SqlType>,
{
}
