use crate::backend::{Backend, DieselReserveSpecialization};
use crate::expression::{Expression, ValidGrouping};
use crate::query_builder::*;
use crate::result::QueryResult;
use crate::sql_types::DieselNumericOps;

#[derive(Debug, Copy, Clone, QueryId, Default, DieselNumericOps, ValidGrouping)]
pub struct Grouped<T>(pub T);

impl<T: Expression> Expression for Grouped<T> {
    type SqlType = T::SqlType;
}

impl<T, DB> QueryFragment<DB> for Grouped<T>
where
    T: QueryFragment<DB>,
    DB: Backend + DieselReserveSpecialization,
{
    fn walk_ast<'b>(&'b self, mut out: AstPass<'_, 'b, DB>) -> QueryResult<()> {
        out.push_sql("(");
        self.0.walk_ast(out.reborrow())?;
        out.push_sql(")");
        Ok(())
    }
}

impl_selectable_expression!(Grouped<T>);
