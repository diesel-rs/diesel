use super::{AstPass, QueryFragment};
use crate::deserialize::{TableQueryable, TableQueryableStmt};
use crate::backend::Backend;
// use crate::expression::subselect::ValidSubselect;
use crate::expression::*;
use crate::query_builder::{QueryId, SelectQuery};
// use crate::query_source::joins::{AppendSelection, Inner, Join};
use crate::query_source::*;
use crate::result::QueryResult;

mod dsl_impls;

/// it is like `SelectStatement` but without
/// `Query`, `ValidSubselect` and `AppendSelection`
#[derive(Debug, Clone, Copy, QueryId)]
#[doc(hidden)]
#[must_use = "Queries are only executed when calling `load`, `get_result` or similar."]
pub struct SelectByStatement<Selection, Statement> {
    pub(crate) select: std::marker::PhantomData<Selection>,
    pub(crate) inner: Statement,
}

impl<S, ST> SelectByStatement<S, ST> {
    #[allow(clippy::too_many_arguments)]
    pub fn new(inner: ST) -> Self {
        SelectByStatement {
            inner, select: Default::default()
        }
    }
}

impl<DB, S, STMT> QueryFragment<DB> for SelectByStatement<S, STMT>
where
    DB: Backend,
    STMT: QueryFragment<DB>,
{
    fn walk_ast(&self, mut out: AstPass<DB>) -> QueryResult<()> {
        self.inner.walk_ast(out.reborrow())?;
        Ok(())
    }
}

impl<S, STMT> SelectQuery for SelectByStatement<S, STMT>
where
    S: TableQueryable,
    STMT: SelectQuery,
{
    type SqlType = STMT::SqlType;
}

/// Allow `SelectStatement<S, Statement>` to act as if it were `Statement`.
impl<S, STMT, T> AppearsInFromClause<T> for SelectByStatement<S, STMT>
where
    STMT: AppearsInFromClause<T>,
{
    type Count = STMT::Count;
}

impl<S, STMT> QuerySource for SelectByStatement<S, STMT>
where
    S: TableQueryable,
    STMT: QuerySource,
    S::Columns: SelectableExpression<Self>,
{
    type FromClause = STMT::FromClause;
    type DefaultSelection = S::Columns;

    fn from_clause(&self) -> Self::FromClause {
        self.inner.from_clause()
    }

    fn default_selection(&self) -> Self::DefaultSelection {
        S::columns()
    }
}

// not implement AppendSelection

impl<S, STMT> TableQueryableStmt for SelectByStatement<S, STMT>
where
    S: TableQueryable,
{
    type Columns = S::Columns;
}
