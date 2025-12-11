// Note: Kinda the same as in insert_statement/batch_insert.rs

use crate::backend::{sql_dialect, Backend, SqlDialect};
use crate::query_builder::{AstPass, QueryFragment, QueryId};
use crate::{QueryResult, Table};
use std::marker::PhantomData;

#[derive(Debug, Clone, Copy, QueryId)]
pub struct ChangesetClause<T, Tab> {
    /// Values to update
    pub values: T,
    _marker: PhantomData<Tab>,
}

pub trait CanUpdateInSingleQuery<DB: Backend> {
    /// How many rows will this query update?
    ///
    /// This function should only return `None` when the query is valid on all
    /// backends, regardless of how many rows get updated.
    fn rows_to_update(&self) -> Option<usize>;
}

pub trait UpdateValues<DB: Backend, T: Table>: QueryFragment<DB> {
    fn column_names(&self, out: AstPass<'_, '_, DB>) -> QueryResult<()>;
}

/// This type represents a batch update clause, which allows
/// to update multiple rows at once.
///
/// Custom backends can specialize the [`QueryFragment`]
/// implementation via [`SqlDialect::BatchUpdateSupport`]
/// or provide fully custom [`ExecuteDsl`](crate::query_dsl::methods::ExecuteDsl)
/// and [`LoadQuery`](crate::query_dsl::methods::LoadQuery) implementations
// #[cfg_attr(
//     feature = "i-implement-a-third-party-backend-and-opt-into-breaking-changes",
//     cfg(feature = "i-implement-a-third-party-backend-and-opt-into-breaking-changes")
// )]
#[derive(Debug)]
pub struct BatchUpdate<V, Tab, QId, const STABLE_QUERY_ID: bool> {
    /// List of values that should be updated
    pub values: V,
    _marker: PhantomData<(QId, Tab)>,
}

impl<V, Tab, QId, const STABLE_QUERY_ID: bool> BatchUpdate<V, Tab, QId, STABLE_QUERY_ID> {
    pub(crate) fn new(values: V) -> Self {
        Self {
            values,
            _marker: PhantomData,
        }
    }
}

impl<V, QId: 'static, Tab: 'static, const STABLE_QUERY_ID: bool> QueryId
    for BatchUpdate<V, Tab, QId, STABLE_QUERY_ID>
{
    type QueryId = QId;

    const HAS_STATIC_QUERY_ID: bool = STABLE_QUERY_ID;
}

impl<T, Table, QId, DB, const HAS_STATIC_QUERY_ID: bool> CanUpdateInSingleQuery<DB>
    for BatchUpdate<T, Table, QId, HAS_STATIC_QUERY_ID>
where
    T: CanUpdateInSingleQuery<DB>,
    DB: Backend,
{
    fn rows_to_update(&self) -> Option<usize> {
        self.values.rows_to_update()
    }
}

impl<T, DB, const N: usize> CanUpdateInSingleQuery<DB> for [T; N]
where
    DB: Backend,
{
    fn rows_to_update(&self) -> Option<usize> {
        Some(N)
    }
}

impl<T, DB, const N: usize> CanUpdateInSingleQuery<DB> for Box<[T; N]>
where
    DB: Backend,
{
    fn rows_to_update(&self) -> Option<usize> {
        Some(N)
    }
}

impl<T, DB> CanUpdateInSingleQuery<DB> for [T]
where
    DB: Backend,
{
    fn rows_to_update(&self) -> Option<usize> {
        Some(self.len())
    }
}

impl<T, DB> CanUpdateInSingleQuery<DB> for Vec<T>
where
    DB: Backend,
{
    fn rows_to_update(&self) -> Option<usize> {
        Some(self.len())
    }
}

// impl<Tab, DB, V, QId, const HAS_STATIC_QUERY_ID: bool> QueryFragment<DB>
//     for BatchUpdate<V, Tab, QId, HAS_STATIC_QUERY_ID>
// where
//     DB: Backend,
//     Self: QueryFragment<DB, DB::BatchUpdateSupport>,
// {
//     fn walk_ast<'b>(&'b self, pass: AstPass<'_, 'b, DB>) -> QueryResult<()> {
//         <Self as QueryFragment<DB, DB::BatchUpdateSupport>>::walk_ast(self, pass)
//     }
// }

// QueryFragment<DB, sql_dialect::batch_update_support::PostgresLikeBatchUpdateSupport>
impl<Tab, DB, V, QId, const HAS_STATIC_QUERY_ID: bool> QueryFragment<DB>
    for BatchUpdate<Vec<ChangesetClause<V, Tab>>, Tab, QId, HAS_STATIC_QUERY_ID>
where
    DB: Backend,
    ChangesetClause<V, Tab>: QueryFragment<DB>,
    V: QueryFragment<DB>,
{
    fn walk_ast<'b>(&'b self, mut out: AstPass<'_, 'b, DB>) -> QueryResult<()> {
        if !HAS_STATIC_QUERY_ID {
            out.unsafe_to_cache_prepared();
        }

        let mut values = self.values.iter();
        if let Some(value) = values.next() {
            value.walk_ast(out.reborrow())?;
        }
        for value in values {
            out.push_sql(", (");
            value.values.walk_ast(out.reborrow())?;
            out.push_sql(")");
        }
        Ok(())
    }
}
