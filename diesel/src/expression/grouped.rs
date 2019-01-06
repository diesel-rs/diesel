use crate::backend::Backend;
use crate::expression::{Expression, NonAggregate};
use crate::query_builder::*;
use crate::result::QueryResult;

#[derive(Debug, Copy, Clone, QueryId, Default, DieselNumericOps)]
pub struct Grouped<T>(pub T);

impl<T: Expression> Expression for Grouped<T> {
    type SqlType = T::SqlType;
}

impl<T: QueryFragment<DB>, DB: Backend> QueryFragment<DB> for Grouped<T> {
    fn walk_ast(&self, mut out: AstPass<DB>) -> QueryResult<()> {
        out.push_sql("(");
        self.0.walk_ast(out.reborrow())?;
        out.push_sql(")");
        Ok(())
    }
}

impl_selectable_expression!(Grouped<T>);

impl<T: NonAggregate> NonAggregate for Grouped<T> where Grouped<T>: Expression {}
