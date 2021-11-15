//! `ONLY` clause.
//!
//! Can only be used on tables, e.g. `my_table::dsl::table.only().select(something)` should generate
//! something like `SELECT something FROM ONLY my_table`.

use crate::pg::Pg;
use crate::prelude::*;
use crate::query_builder::{AstPass, QueryFragment, QueryId};

/// Represents a query with an `ONLY` clause.
#[derive(Debug, QueryId)]
pub struct Only<T> {
    pub(crate) query: T,
}

impl<T> QueryFragment<Pg> for Only<T>
where
    T: QueryFragment<Pg>,
{
    fn walk_ast<'a: 'b, 'b>(&'a self, mut out: AstPass<'_, 'b, Pg>) -> QueryResult<()> {
        out.push_sql("ONLY ");
        self.query.walk_ast(out.reborrow())?;
        Ok(())
    }
}

impl<T> QuerySource for Only<T>
where
    T: QuerySource,
    T::DefaultSelection: SelectableExpression<Only<T>>,
{
    type FromClause = Only<T::FromClause>;
    type DefaultSelection = T::DefaultSelection;

    fn from_clause(&self) -> Self::FromClause {
        Only {
            query: self.query.from_clause(),
        }
    }
    fn default_selection(&self) -> Self::DefaultSelection {
        self.query.default_selection()
    }
}
