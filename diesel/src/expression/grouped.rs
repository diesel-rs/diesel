use backend::Backend;
use expression::{Expression, NonAggregate};
use insertable::InsertValues;
use query_builder::*;
use query_source::Table;
use result::QueryResult;

#[derive(Debug, Copy, Clone, QueryId, Default)]
pub struct Grouped<T>(pub T);

impl<T: Expression> Expression for Grouped<T> {
    type SqlType = T::SqlType;
}

impl<T: QueryFragment<DB>, DB: Backend> QueryFragment<DB> for Grouped<T> {
    fn walk_ast(&self, mut out: AstPass<DB>) -> QueryResult<()> {
        let is_noop = self.0.is_noop()?;
        if !is_noop {
            out.push_sql("(");
        }
        self.0.walk_ast(out.reborrow())?;
        if !is_noop {
            out.push_sql(")");
        }
        Ok(())
    }
}

impl_selectable_expression!(Grouped<T>);

impl<T: NonAggregate> NonAggregate for Grouped<T>
where
    Grouped<T>: Expression,
{
}

impl<T, U, DB> InsertValues<T, DB> for Grouped<U>
where
    T: Table,
    DB: Backend,
    U: InsertValues<T, DB>,
{
    fn column_names(&self, out: AstPass<DB>) -> QueryResult<()> {
        self.0.column_names(out)
    }
}

impl<T, U> UndecoratedInsertRecord<T> for Grouped<U>
where
    U: UndecoratedInsertRecord<T>,
{
}
