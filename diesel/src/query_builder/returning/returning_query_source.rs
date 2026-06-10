//! The `ReturningQuerySource<StmtKind, T>` wrapper used to type-check `RETURNING`
//! clauses, the statement-kind markers it is parameterized over, and the
//! [`InsertStmtKind`] dispatch trait.
//!
//! This module holds the items needed to type-check `RETURNING` clauses that are not specific
//! to a particular backend.

use crate::expression::{AppearsOnTable, BoxableExpression, Expression, SelectableExpression};
use crate::query_source::joins::ToInnerJoin;
use crate::query_source::{AppearsInFromClause, Never, QueryRelation, QuerySource, TableNotEqual};
use alloc::boxed::Box;
use core::marker::PhantomData;

/// Statement-kind marker
#[derive(Debug, Copy, Clone)]
pub struct InsertStmtWithoutOnConflictDoUpdate;

/// Statement-kind marker
#[derive(Debug, Copy, Clone)]
pub struct UpdateStmt;

/// Statement-kind marker
#[derive(Debug, Copy, Clone)]
pub struct DeleteStmt;

/// Statement-kind marker
#[derive(Debug, Copy, Clone)]
pub struct InsertStmtWithOnConflictDoUpdate;

/// Synthetic query source used as the `QS` parameter of
/// [`SelectableExpression`](crate::expression::SelectableExpression) /
/// [`AppearsOnTable`](crate::expression::AppearsOnTable) when type-checking
/// `RETURNING` clauses.
///
/// `StmtKind` is one of the marker structs above; `T` is the table that the
/// `INSERT`/`UPDATE`/`DELETE` is acting on.
#[derive(Debug, Clone, Copy)]
pub struct ReturningQuerySource<StmtKind, T>(PhantomData<(StmtKind, T)>);

impl<StmtKind, T> QuerySource for ReturningQuerySource<StmtKind, T>
where
    T: QuerySource + Default,
    T::DefaultSelection: SelectableExpression<Self>,
{
    type FromClause = T::FromClause;
    type DefaultSelection = T::DefaultSelection;

    fn from_clause(&self) -> Self::FromClause {
        T::default().from_clause()
    }

    fn default_selection(&self) -> Self::DefaultSelection {
        T::default().default_selection()
    }
}

impl<StmtKind, T1, T2> AppearsInFromClause<T1> for ReturningQuerySource<StmtKind, T2>
where
    T1: TableNotEqual<T2> + QueryRelation,
    T2: QueryRelation,
{
    type Count = Never;
}
// For typechecking of `old(column).nullable()`

impl<T> ToInnerJoin for ReturningQuerySource<UpdateStmt, T> {
    type InnerJoin = Self;
}

impl<T> ToInnerJoin for ReturningQuerySource<DeleteStmt, T> {
    type InnerJoin = Self;
}

impl<T> ToInnerJoin for ReturningQuerySource<InsertStmtWithoutOnConflictDoUpdate, T> {
    type InnerJoin = Self;
}

// `InsertStmtWithOnConflictDoUpdate` maps to `UpdateStmt`. This is what makes
// `Nullable<Old<C>>: SelectableExpression<ReturningQuerySource<InsertStmtWithOnConflictDoUpdate, _>>`
// resolve through the existing
// `impl<T, QS> SelectableExpression<QS> for Nullable<T>
//     where QS: ToInnerJoin, T: SelectableExpression<QS::InnerJoin>` impl
// (see `crate::expression::nullable::Nullable`): that bound reduces to
// `Old<C>: SelectableExpression<ReturningQuerySource<UpdateStmt, _>>`, which
// holds, while the bare `Old<C>: SelectableExpression<ReturningQuerySource<InsertStmtWithOnConflictDoUpdate, _>>`
// is intentionally *not* implemented — forcing users to write
// `old(col).nullable()` in `ON CONFLICT ... DO UPDATE` for type safety.
impl<T> ToInnerJoin for ReturningQuerySource<InsertStmtWithOnConflictDoUpdate, T> {
    type InnerJoin = ReturningQuerySource<UpdateStmt, T>;
}

/// Maps an `InsertStatement` `Values` shape to the statement-kind marker that
/// should be used when type-checking that statement's `RETURNING` clause.
///
/// This is what makes `RETURNING old(col)` accept `INSERT ... ON CONFLICT
/// ... DO UPDATE` (where the marker is [`InsertStmtWithOnConflictDoUpdate`], for
/// which `Nullable<Old<C>>` is a valid `RETURNING` element) but reject plain
/// `INSERT` (where the marker is [`InsertStmtWithoutOnConflictDoUpdate`], for which `Old<C>` does not
/// implement `SelectableExpression`).
///
/// The trait is sealed in spirit — it only has impls for the values shapes
/// `diesel` itself produces — but it is exposed publicly under the
/// `i-implement-a-third-party-backend-and-opt-into-breaking-changes` feature
/// so third-party backends that introduce new values shapes can add their own
/// impls.
pub trait InsertStmtKind {
    /// The statement-kind marker (see e.g. [`InsertStmtWithoutOnConflictDoUpdate`],
    /// [`InsertStmtWithOnConflictDoUpdate`]) used as the `StmtKind` parameter of
    /// [`ReturningQuerySource`] for `INSERT` statements with this `Values`
    /// shape.
    type StmtKind;
}

impl<T, Tab> InsertStmtKind for crate::query_builder::ValuesClause<T, Tab> {
    type StmtKind = InsertStmtWithoutOnConflictDoUpdate;
}

impl<V, Tab, QId, const STABLE_QUERY_ID: bool> InsertStmtKind
    for crate::query_builder::BatchInsert<V, Tab, QId, STABLE_QUERY_ID>
{
    type StmtKind = InsertStmtWithoutOnConflictDoUpdate;
}

impl InsertStmtKind for crate::query_builder::insert_statement::DefaultValues {
    type StmtKind = InsertStmtWithoutOnConflictDoUpdate;
}

impl<S, C> InsertStmtKind for crate::query_builder::insert_statement::InsertFromSelect<S, C> {
    type StmtKind = InsertStmtWithoutOnConflictDoUpdate;
}

impl<V, Target, T, WhereClause> InsertStmtKind
    for crate::query_builder::upsert::on_conflict_clause::OnConflictValues<
        V,
        Target,
        crate::query_builder::upsert::on_conflict_actions::DoNothing<T>,
        WhereClause,
    >
{
    // ON CONFLICT DO NOTHING does not return the lines that conflicted if using RETURNING
    // so it's unnecessary (and would even be misleading) to allow RETURNING old.xxx
    // in this case.
    type StmtKind = InsertStmtWithoutOnConflictDoUpdate;
}

impl<V, Target, Changeset, Tab, WhereClause> InsertStmtKind
    for crate::query_builder::upsert::on_conflict_clause::OnConflictValues<
        V,
        Target,
        crate::query_builder::upsert::on_conflict_actions::DoUpdate<Changeset, Tab>,
        WhereClause,
    >
{
    type StmtKind = InsertStmtWithOnConflictDoUpdate;
}

/// Make `Box<dyn BoxableExpression<table>>` be `SelectableExpression` in returning clauses.
/// This is necessary for backwards-compatibility.
///
/// Unfortunately we cannot implement this directly for `dyn BoxableExpression`
/// (and rely on our existing generic impls for `Box<T>`) because the compiler
/// complains that there is already an automatic implementation of `SelectableExpression` for
/// `dyn BoxableExpression`, even though the `QS` type is different.
/// As a workaround, we implement it for `Box<dyn BoxableExpression<table>>`, which should
/// cover for most users.
impl<'a, QS, ST, DB, GB, IsAggregate, StmtKind>
    SelectableExpression<ReturningQuerySource<StmtKind, QS>>
    for Box<dyn BoxableExpression<QS, DB, GB, IsAggregate, SqlType = ST> + 'a>
where
    Box<dyn BoxableExpression<QS, DB, GB, IsAggregate, SqlType = ST> + 'a>: Expression,
{
}
/// See comment on `SelectableExpression` impl above.
impl<'a, QS, ST, DB, GB, IsAggregate, StmtKind> AppearsOnTable<ReturningQuerySource<StmtKind, QS>>
    for Box<dyn BoxableExpression<QS, DB, GB, IsAggregate, SqlType = ST> + 'a>
where
    Box<dyn BoxableExpression<QS, DB, GB, IsAggregate, SqlType = ST> + 'a>: Expression,
{
}
