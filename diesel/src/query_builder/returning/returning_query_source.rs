//! The `ReturningQuerySource<StmtKind, T>` wrapper used to type-check `RETURNING`
//! clauses, the statement-kind markers it is parameterized over, and the
//! [`InsertStmtKind`] dispatch trait.
//!
//! See [`ReturningQuerySource`] for the high-level picture.

use crate::query_source::joins::ToInnerJoin;
use crate::query_source::{AppearsInFromClause, Once};
use core::marker::PhantomData;

/// Marker trait used to drive `RETURNING`-clause type-checking errors.
///
/// This trait is implemented for an expression `E` exactly when `E` is a
/// valid leaf for the `RETURNING` clause of a `Stmt`-kind statement on
/// `Table`. It is plugged in as a `where`-clause on every per-column /
/// per-leaf `SelectableExpression<ReturningQuerySource<Stmt, Table>>` impl
/// — i.e. those impls are kept generic in `Stmt` and `Table`, and the
/// where-clause `Self: ValidInReturningOf<Stmt, Table>` is what actually
/// constrains them. When the leaf isn't valid, the resulting error message
/// is anchored on this trait, whose [`#[diagnostic::on_unimplemented]`]
/// substitutes `{Stmt}` and `{Table}` *separately* (each with its own
/// pretty-print budget) so neither argument is collapsed to `...` the way a
/// single `ReturningQuerySource<Stmt, Table>` slot can be in long
/// `INSERT ... ON CONFLICT ...` chains.
///
/// The same idea is used elsewhere in `diesel` — see
/// [`crate::expression::operators::LikeIsAllowedForType`] for a precedent.
#[diagnostic::on_unimplemented(
    message = "`{Self}` cannot appear in the `RETURNING` clause of a `{StmtKind}` on `{Table}`",
    note = "`{Self}` is not a valid `RETURNING` element for this statement kind on this table",
    note = "for `INSERT ... ON CONFLICT ... DO UPDATE`, wrap with `.nullable()` so freshly-inserted rows can return `NULL` for `old.col`"
)]
pub trait ValidInReturningOf<Table, StmtKind> {}

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
/// [`SelectableExpression`] / [`AppearsOnTable`] when type-checking
/// `RETURNING` clauses.
///
/// `StmtKind` is one of the marker structs above; `T` is the table that the
/// `INSERT`/`UPDATE`/`DELETE` is acting on.
#[derive(Debug, Clone, Copy)]
pub struct ReturningQuerySource<StmtKind, T>(PhantomData<(StmtKind, T)>);

impl<StmtKind, T> AppearsInFromClause<T> for ReturningQuerySource<StmtKind, T> {
    type Count = Once;
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
