use super::ValuesClause;
use crate::backend::{sql_dialect, Backend, SqlDialect};
use crate::insertable::CanInsertInSingleQuery;
use crate::query_builder::{AstPass, QueryFragment, QueryId};
use crate::{Insertable, QueryResult};
use std::marker::PhantomData;

#[doc(hidden)]
#[derive(Debug)]
pub struct BatchInsert<V, Tab, QId, const STABLE_QUERY_ID: bool> {
    pub values: V,
    _marker: PhantomData<(QId, Tab)>,
}

impl<V, Tab, QId, const STABLE_QUERY_ID: bool> BatchInsert<V, Tab, QId, STABLE_QUERY_ID> {
    pub(crate) fn new(values: V) -> Self {
        Self {
            values,
            _marker: PhantomData,
        }
    }
}

impl<V, QId: 'static, Tab: 'static, const STABLE_QUERY_ID: bool> QueryId
    for BatchInsert<V, Tab, QId, STABLE_QUERY_ID>
{
    type QueryId = QId;

    const HAS_STATIC_QUERY_ID: bool = STABLE_QUERY_ID;
}

impl<T, Table, QId, DB, const HAS_STATIC_QUERY_ID: bool> CanInsertInSingleQuery<DB>
    for BatchInsert<T, Table, QId, HAS_STATIC_QUERY_ID>
where
    T: CanInsertInSingleQuery<DB>,
DB: Backend+ SqlDialect<InsertWithDefaultKeyword = sql_dialect::default_keyword_for_insert::IsoSqlDefaultKeyword>
{
    fn rows_to_insert(&self) -> Option<usize> {
        self.values.rows_to_insert()
    }
}

impl<T, DB, const N: usize> CanInsertInSingleQuery<DB> for [T; N]
where
DB: Backend+ SqlDialect<InsertWithDefaultKeyword = sql_dialect::default_keyword_for_insert::IsoSqlDefaultKeyword>
{
    fn rows_to_insert(&self) -> Option<usize> {
        Some(N)
    }
}

impl<T, DB, const N: usize> CanInsertInSingleQuery<DB> for Box<[T; N]>
where
    DB: Backend+ SqlDialect<InsertWithDefaultKeyword = sql_dialect::default_keyword_for_insert::IsoSqlDefaultKeyword>
{
    fn rows_to_insert(&self) -> Option<usize> {
        Some(N)
    }
}

impl<T, DB> CanInsertInSingleQuery<DB> for [T]
where
    DB: Backend+ SqlDialect<InsertWithDefaultKeyword = sql_dialect::default_keyword_for_insert::IsoSqlDefaultKeyword>
{
    fn rows_to_insert(&self) -> Option<usize> {
        Some(self.len())
    }
}

impl<T, DB> CanInsertInSingleQuery<DB> for Vec<T>
where
    DB: Backend+ SqlDialect<InsertWithDefaultKeyword = sql_dialect::default_keyword_for_insert::IsoSqlDefaultKeyword>,
{
    fn rows_to_insert(&self) -> Option<usize> {
        Some(self.len())
    }
}

#[doc(hidden)]
pub trait AsValueIterator<Tab> {
    type Item;

    // we return a boxed iterator as
    // a plain iterator may involve lifetimes
    // and the trait itself cannot have an attached lifetime
    // therefor it wouldn't be possible to name the type correctly
    // FIXME: This allocation can be removed if GAT's are stable
    fn as_value_iter<'a>(
        &'a self,
    ) -> Box<dyn Iterator<Item = <&'a Self::Item as Insertable<Tab>>::Values> + 'a>
    where
        Self::Item: 'a,
        &'a Self::Item: Insertable<Tab>;
}

// https://github.com/rust-lang/rust-clippy/issues/7497
#[allow(clippy::redundant_closure)]
impl<T, Tab, const N: usize> AsValueIterator<Tab> for [T; N] {
    type Item = T;

    fn as_value_iter<'a>(
        &'a self,
    ) -> Box<dyn Iterator<Item = <&'a T as Insertable<Tab>>::Values> + 'a>
    where
        Self::Item: 'a,
        &'a T: Insertable<Tab>,
    {
        Box::new(IntoIterator::into_iter(self).map(|v| Insertable::values(v)))
    }
}

// https://github.com/rust-lang/rust-clippy/issues/7497
#[allow(clippy::redundant_closure)]
impl<'b, T, Tab, const N: usize> AsValueIterator<Tab> for &'b [T; N] {
    type Item = T;

    fn as_value_iter<'a>(
        &'a self,
    ) -> Box<dyn Iterator<Item = <&'a Self::Item as Insertable<Tab>>::Values> + 'a>
    where
        Self::Item: 'a,
        &'a T: Insertable<Tab>,
    {
        Box::new(IntoIterator::into_iter(*self).map(|v| Insertable::values(v)))
    }
}

// https://github.com/rust-lang/rust-clippy/issues/7497
#[allow(clippy::redundant_closure)]
impl<T, Tab, const N: usize> AsValueIterator<Tab> for Box<[T; N]> {
    type Item = T;

    fn as_value_iter<'a>(
        &'a self,
    ) -> Box<dyn Iterator<Item = <&'a Self::Item as Insertable<Tab>>::Values> + 'a>
    where
        Self::Item: 'a,
        &'a T: Insertable<Tab>,
    {
        Box::new(IntoIterator::into_iter(&**self).map(|v| Insertable::values(v)))
    }
}

// https://github.com/rust-lang/rust-clippy/issues/7497
#[allow(clippy::redundant_closure)]
impl<T, Tab> AsValueIterator<Tab> for Vec<T> {
    type Item = T;

    fn as_value_iter<'a>(
        &'a self,
    ) -> Box<dyn Iterator<Item = <&'a Self::Item as Insertable<Tab>>::Values> + 'a>
    where
        Self::Item: 'a,
        &'a T: Insertable<Tab>,
    {
        Box::new(IntoIterator::into_iter(self).map(|v| Insertable::values(v)))
    }
}

// https://github.com/rust-lang/rust-clippy/issues/7497
#[allow(clippy::redundant_closure)]
impl<'b, T, Tab> AsValueIterator<Tab> for &'b [T] {
    type Item = T;

    fn as_value_iter<'a>(
        &'a self,
    ) -> Box<dyn Iterator<Item = <&'a Self::Item as Insertable<Tab>>::Values> + 'a>
    where
        Self::Item: 'a,
        &'a T: Insertable<Tab>,
    {
        Box::new(IntoIterator::into_iter(*self).map(|v| Insertable::values(v)))
    }
}

// This trait is a workaround to issues with hrtb (for<'a>) in rust
// Rustc refuses to unify associated types pulled out of some trait
// bound that involves a hrtb. As a workaround we create this
// intermediate private trait and hide all "advanced" restrictions
// behind this trait
// For this case this is essentially only that `Self::Values` implement `QueryFragment`
// This allows us to unify all that behind a single trait bound in the
// `QueryFragment` impl below
#[doc(hidden)]
pub trait InsertableQueryfragment<Tab, DB>
where
    Self: Insertable<Tab>,
    DB: Backend,
{
    fn walk_ast_helper_with_value_clause(values: Self::Values, out: AstPass<DB>)
        -> QueryResult<()>;

    fn walk_ast_helper_without_value_clause(
        values: Self::Values,
        out: AstPass<DB>,
    ) -> QueryResult<()>;
}

impl<'a, Tab, DB, T> InsertableQueryfragment<Tab, DB> for &'a T
where
    Self: Insertable<Tab>,
    <&'a T as Insertable<Tab>>::Values: QueryFragment<DB> + IsValuesClause<DB>,
    DB: Backend,
{
    fn walk_ast_helper_with_value_clause(
        values: Self::Values,
        out: AstPass<DB>,
    ) -> QueryResult<()> {
        values.walk_ast(out)
    }

    fn walk_ast_helper_without_value_clause(
        values: Self::Values,
        out: AstPass<DB>,
    ) -> QueryResult<()> {
        values.values().walk_ast(out)
    }
}

#[doc(hidden)]
pub trait IsValuesClause<DB: Backend> {
    type Inner: QueryFragment<DB>;

    fn values(&self) -> &Self::Inner;
}

impl<Inner, Tab, DB> IsValuesClause<DB> for ValuesClause<Inner, Tab>
where
    DB: Backend,
    Inner: QueryFragment<DB>,
{
    type Inner = Inner;

    fn values(&self) -> &Self::Inner {
        &self.values
    }
}

impl<Tab, DB, V, QId, const HAS_STATIC_QUERY_ID: bool> QueryFragment<DB>
    for BatchInsert<V, Tab, QId, HAS_STATIC_QUERY_ID>
where
    DB: Backend,
    Self: QueryFragment<DB, DB::BatchInsertSupport>,
{
    fn walk_ast(&self, pass: AstPass<DB>) -> QueryResult<()> {
        <Self as QueryFragment<DB, DB::BatchInsertSupport>>::walk_ast(self, pass)
    }
}

impl<Tab, DB, T, V, QId, const HAS_STATIC_QUERY_ID: bool>
    QueryFragment<DB, sql_dialect::batch_insert_support::PostgresLikeBatchInsertSupport>
    for BatchInsert<V, Tab, QId, HAS_STATIC_QUERY_ID>
where
    DB: Backend
        + SqlDialect<
            BatchInsertSupport = sql_dialect::batch_insert_support::PostgresLikeBatchInsertSupport,
        >,
    DB::InsertWithDefaultKeyword: sql_dialect::default_keyword_for_insert::SupportsDefaultKeyword,
    V: AsValueIterator<Tab, Item = T>,
    for<'a> &'a T: InsertableQueryfragment<Tab, DB>,
{
    fn walk_ast(&self, mut out: AstPass<DB>) -> QueryResult<()> {
        if !HAS_STATIC_QUERY_ID {
            out.unsafe_to_cache_prepared();
        }

        let mut values = self.values.as_value_iter();
        if let Some(value) = values.next() {
            <&T as InsertableQueryfragment<Tab, DB>>::walk_ast_helper_with_value_clause(
                value,
                out.reborrow(),
            )?;
        }
        for value in values {
            out.push_sql(", (");
            <&T as InsertableQueryfragment<Tab, DB>>::walk_ast_helper_without_value_clause(
                value,
                out.reborrow(),
            )?;
            out.push_sql(")");
        }
        Ok(())
    }
}
