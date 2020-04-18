use super::{AstPass, QueryFragment};
use crate::backend::Backend;
use crate::connection::Connection;
use crate::deserialize::{QueryableByName, TableQueryable};
// use crate::expression::subselect::ValidSubselect;
use crate::expression::*;
use crate::query_builder::{QueryId, SelectQuery};
use crate::query_dsl::LoadQuery;
// use crate::query_source::joins::{AppendSelection, Inner, Join};
use crate::query_source::*;
use crate::result::QueryResult;

mod dsl_impls;

/// it is like `SelectStatement` but without
/// `Query`, `ValidSubselect` and `AppendSelection`
#[derive(Debug, Clone, Copy)]
#[doc(hidden)]
#[must_use = "Queries are only executed when calling `load`, `get_result` or similar."]
pub struct SelectByStatement<Selection, Statement> {
    pub(crate) select: std::marker::PhantomData<Selection>,
    pub(crate) inner: Statement,
}

impl<S, STMT> QueryId for SelectByStatement<S, STMT>
where
    STMT: QueryId
{
    type QueryId = SelectByStatement<(), STMT::QueryId>;
    const HAS_STATIC_QUERY_ID: bool = STMT::HAS_STATIC_QUERY_ID;
}

impl<S, ST> SelectByStatement<S, ST> {
    pub(crate) fn new(inner: ST) -> Self {
        SelectByStatement {
            inner,
            select: Default::default(),
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

impl<ST, S, STMT> SelectQuery for SelectByStatement<S, STMT>
where
    S: TableQueryable,
    S::Columns: Expression<SqlType = ST>,
    STMT: SelectQuery<SqlType = ST>,
{
    // TODO: check SelectByClause
    type SqlType = ST;
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

impl<Conn, S, ST> LoadQuery<Conn, S> for SelectByStatement<S, ST>
where
    Conn: Connection,
    S: QueryableByName<Conn::Backend> + TableQueryable,
    Self: QueryFragment<Conn::Backend> + SelectQuery + QueryId,
{
    fn internal_load(self, conn: &Conn) -> QueryResult<Vec<S>> {
        conn.query_by_name(&self)
    }
}
