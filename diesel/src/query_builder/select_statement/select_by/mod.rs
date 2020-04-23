use std::fmt;
use std::marker::PhantomData;

use super::{AstPass, QueryFragment};
use crate::backend::Backend;
use crate::connection::Connection;
use crate::expression::subselect::ValidSubselect;
use crate::expression::*;
use crate::query_builder::{QueryId, SelectByQuery, SelectQuery};
use crate::query_dsl::LoadQuery;
use crate::query_source::*;
use crate::result::QueryResult;

mod dsl_impls;

/// It is like `SelectStatement` but without
/// `Query`/`AsQuery`, `AppendSelection` and `IntoUpdateTarget`.
/// note: it would have conflicting implementation of `LoadQuery` if
/// this implements `AsQuery`.
/// It does not support `GroupByDsl` right now (using these dsl would
/// unboxing into inner statement).
/// When using `SelectDsl`/`SelectByDsl` would intendedly unbox/rebox
/// Other operation should keep Selection invariant.
#[doc(hidden)]
#[must_use = "Queries are only executed when calling `load`, `get_result` or similar."]
pub struct SelectByStatement<Selection, Statement> {
    pub(crate) select: PhantomData<Selection>,
    pub(crate) inner: Statement,
}

impl<S, Stmt> fmt::Debug for SelectByStatement<S, Stmt>
where
    Stmt: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SelectByStatement")
            .field("inner", &self.inner)
            .finish()
    }
}

impl<S, Stmt> Clone for SelectByStatement<S, Stmt>
where
    Stmt: Clone,
{
    fn clone(&self) -> Self {
        SelectByStatement::new(self.inner.clone())
    }
}

impl<S, Stmt> Copy for SelectByStatement<S, Stmt> where Stmt: Copy {}

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
    Stmt: SelectQuery<SqlType = ST>,
{
    type SqlType = ST;
}

impl<S, Stmt> SelectByQuery for SelectByStatement<S, Stmt>
where
    S: Selectable,
    Stmt: SelectByQuery<Expression = S::Expression>,
{
    type Expression = S::Expression;
}

impl<S, Stmt, QS> ValidSubselect<QS> for SelectByStatement<S, Stmt>
where
    Self: SelectQuery,
    Stmt: ValidSubselect<QS>,
{
}

impl<S, Stmt, T> AppearsInFromClause<T> for SelectByStatement<S, Stmt>
where
    Stmt: AppearsInFromClause<T>,
{
    type Count = Stmt::Count;
}

impl<S, Stmt> QuerySource for SelectByStatement<S, Stmt>
where
    S: Selectable,
    Stmt: QuerySource,
    S::Expression: SelectableExpression<Self>,
{
    type FromClause = Stmt::FromClause;
    type DefaultSelection = S::Expression;

    fn from_clause(&self) -> Self::FromClause {
        self.inner.from_clause()
    }

    fn default_selection(&self) -> Self::DefaultSelection {
        S::new_expression()
    }
}

impl<Conn, S, Stmt> LoadQuery<Conn, S> for SelectByStatement<S, Stmt>
where
    Conn: Connection,
    Self: SelectByQuery,
    Stmt: LoadQuery<Conn, S>,
{
    fn internal_load(self, conn: &Conn) -> QueryResult<Vec<S>> {
        self.inner.internal_load(conn)
    }
}
