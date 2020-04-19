use super::{AstPass, QueryFragment};
use crate::backend::Backend;
use crate::connection::Connection;
use crate::deserialize::{QueryableByName, TableQueryable};
use crate::expression::*;
use crate::query_builder::{QueryId, SelectByQuery, SelectQuery};
use crate::query_dsl::LoadQuery;
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

impl<S, Stmt> QueryId for SelectByStatement<S, Stmt>
where
    Stmt: QueryId,
{
    type QueryId = Stmt::QueryId;
    const HAS_STATIC_QUERY_ID: bool = Stmt::HAS_STATIC_QUERY_ID;
}

impl<S, ST> SelectByStatement<S, ST> {
    pub(crate) fn new(inner: ST) -> Self {
        SelectByStatement {
            inner,
            select: Default::default(),
        }
    }
}

impl<DB, S, Stmt> QueryFragment<DB> for SelectByStatement<S, Stmt>
where
    DB: Backend,
    Stmt: QueryFragment<DB>,
{
    fn walk_ast(&self, mut out: AstPass<DB>) -> QueryResult<()> {
        self.inner.walk_ast(out.reborrow())?;
        Ok(())
    }
}

impl<ST, S, Stmt> SelectQuery for SelectByStatement<S, Stmt>
where
    S: TableQueryable,
    S::Columns: Expression<SqlType = ST>,
    Stmt: SelectQuery<SqlType = ST>,
{
    // TODO: check SelectByClause
    type SqlType = ST;
}

impl<S, Stmt> SelectByQuery for SelectByStatement<S, Stmt>
where
    S: TableQueryable,
    Stmt: SelectByQuery<Columns = S::Columns>,
{
    type Columns = S::Columns;
}

/// Allow `SelectStatement<S, Statement>` to act as if it were `Statement`.
impl<S, Stmt, T> AppearsInFromClause<T> for SelectByStatement<S, Stmt>
where
    Stmt: AppearsInFromClause<T>,
{
    type Count = Stmt::Count;
}

impl<S, Stmt> QuerySource for SelectByStatement<S, Stmt>
where
    S: TableQueryable,
    Stmt: QuerySource,
    S::Columns: SelectableExpression<Self>,
{
    type FromClause = Stmt::FromClause;
    type DefaultSelection = S::Columns;

    fn from_clause(&self) -> Self::FromClause {
        self.inner.from_clause()
    }

    fn default_selection(&self) -> Self::DefaultSelection {
        S::columns()
    }
}

// not implement AppendSelection

impl<Conn, S, Stmt, CL, ST> LoadQuery<Conn, S> for SelectByStatement<S, Stmt>
where
    Conn: Connection,
    CL: Expression<SqlType = ST>,
    S: QueryableByName<Conn::Backend> + TableQueryable<Columns = CL>,
    Self: QueryFragment<Conn::Backend> + SelectByQuery<Columns = CL> + QueryId,
    Stmt: SelectQuery<SqlType = ST>,
{
    fn internal_load(self, conn: &Conn) -> QueryResult<Vec<S>> {
        conn.query_by_name(&self)
    }
}
