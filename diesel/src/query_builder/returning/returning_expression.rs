//! Type-checking traits and statement-kind markers for `RETURNING` clauses.
//!
//! See [`ReturningExpression`] for the high-level picture.

use crate::expression::{Expression, SelectableExpression};
use crate::sql_types::SingleValue;

/// Statement-kind marker used as the `Stmt` parameter of
/// [`ReturningExpression`] for `INSERT` statements (other than
/// `INSERT ... ON CONFLICT ... DO UPDATE`, which uses
/// [`InsertOnConflictDoUpdateStmt`]).
///
/// This exists so that `RETURNING` typechecking can vary per statement kind
/// (relevant in PostgreSQL 18 and later, where `RETURNING old.col` is allowed
/// for some but not all statements). It is deliberately **separate** from the
/// SQL-keyword markers `Insert`/`InsertOrIgnore`/`Replace`, which control the
/// emitted keyword rather than the row-shape of `RETURNING`.
#[allow(missing_copy_implementations)]
#[derive(Debug)]
pub struct InsertStmt;

/// Statement-kind marker used as the `Stmt` parameter of
/// [`ReturningExpression`] for `UPDATE` statements.
///
/// Together with [`InsertOnConflictDoUpdateStmt`], this is one of the markers
/// for which [`crate::pg::returning::Old`] implements [`ReturningExpression`],
/// which is what makes `.returning(old(col))` compile-time exclusive to
/// `UPDATE` and `INSERT ... ON CONFLICT ... DO UPDATE`.
#[allow(missing_copy_implementations)]
#[derive(Debug)]
pub struct UpdateStmt;

/// Statement-kind marker used as the `Stmt` parameter of
/// [`ReturningExpression`] for `DELETE` statements.
#[allow(missing_copy_implementations)]
#[derive(Debug)]
pub struct DeleteStmt;

/// Statement-kind marker used as the `Stmt` parameter of
/// [`ReturningExpression`] for `INSERT ... ON CONFLICT ... DO UPDATE`
/// statements.
///
/// This is split out from [`InsertStmt`] because the row-shape of `RETURNING`
/// is different in this case: PostgreSQL 18+ accepts `RETURNING old.col` here,
/// but the value of `old.col` is `NULL` for rows that were inserted (rather
/// than updated), so the resulting Rust SQL type is `Nullable<SqlType>` even
/// when the column itself is `NOT NULL`.
#[allow(missing_copy_implementations)]
#[derive(Debug)]
pub struct InsertOnConflictDoUpdateStmt;

/// Marks expressions that may appear in a `RETURNING` clause for a given
/// statement kind / target table combination.
///
/// This trait replaces a bare `SelectableExpression<Table>` bound on the
/// `RETURNING` lists of `INSERT`/`UPDATE`/`DELETE`. The extra `Stmt` parameter
/// lets the `SqlType` of an expression genuinely depend on which statement it
/// is being used inside — necessary because PostgreSQL's `RETURNING old.col`
/// produces different result types in different statement contexts.
///
/// The only place where the `Stmt` parameter is observable is in
/// [`crate::pg::returning::Old`]: it is a `ReturningExpression<UpdateStmt, _>`
/// (with the column's natural `SqlType`) and a
/// `ReturningExpression<InsertOnConflictDoUpdateStmt, _>` (with that `SqlType`
/// wrapped in [`crate::sql_types::Nullable`]). This is the mechanism by which
/// `.returning(old(col))` is admitted only in `UPDATE` and
/// `INSERT ... ON CONFLICT ... DO UPDATE`, and rejected at compile time in
/// plain `INSERT` and `DELETE`.
///
/// # Existing returning expressions
///
/// Any type that implements [`SelectableExpression<Table>`] with a single-value
/// SQL type (i.e. anything but a tuple type) is automatically a
/// `ReturningExpression<Stmt, Table>` for every `Stmt`, via a blanket impl. So
/// existing user code that calls `.returning(...)` does not change.
///
/// # Tuples
///
/// Tuples of `ReturningExpression<Stmt, T>` are also `ReturningExpression<Stmt, T>`
/// — including tuples mixing [`crate::pg::returning::Old`] with plain columns
/// in the `Stmt = UpdateStmt` / `Stmt = InsertOnConflictDoUpdateStmt` cases.
/// The blanket impl is disambiguated from the tuple impl using the
/// [`SingleValue`] marker trait, which tuple SQL types never implement.
///
/// [`SelectableExpression<Table>`]: crate::expression::SelectableExpression
#[diagnostic::on_unimplemented(
    message = "`{Self}` cannot appear in the `RETURNING` clause of this statement",
    note = "for `INSERT`/`UPDATE`/`DELETE` `RETURNING` lists, every element \n\
            must be a column, or a tuple of columns / `RETURNING`-valid \n\
            expressions, that belongs to the modified table.\n\
            `old(col)` is only valid in `UPDATE` and `INSERT ... ON CONFLICT \n\
            ... DO UPDATE` `RETURNING` lists.",
    label = "expression `{Self}` is not a valid `RETURNING` element here"
)]
pub trait ReturningExpression<Stmt, Table> {
    /// The Rust SQL type produced by this expression in the given statement
    /// context. For non-`Old<C>` expressions this is the same as the
    /// expression's [`Expression::SqlType`].
    type SqlType;
}

#[diagnostic::do_not_recommend]
impl<E, Stmt, T, ST> ReturningExpression<Stmt, T> for E
where
    E: SelectableExpression<T> + Expression<SqlType = ST>,
    ST: SingleValue,
{
    type SqlType = ST;
}

/// Maps an `InsertStatement` `Values` shape to the statement-kind marker that
/// should be used when type-checking that statement's `RETURNING` clause.
///
/// This is what makes `RETURNING old(col)` accept `INSERT ... ON CONFLICT
/// ... DO UPDATE` (where the marker is [`InsertOnConflictDoUpdateStmt`]) but
/// reject plain `INSERT` (where the marker is [`InsertStmt`], for which
/// `Old<C>` does not implement `ReturningExpression`).
///
/// The trait is sealed in spirit — it only has impls for the values shapes
/// `diesel` itself produces — but it is exposed publicly under the
/// `i-implement-a-third-party-backend-and-opt-into-breaking-changes` feature
/// so third-party backends that introduce new values shapes can add their own
/// impls.
pub trait InsertStmtKind {
    /// The statement-kind marker (see e.g. [`InsertStmt`],
    /// [`InsertOnConflictDoUpdateStmt`]) used as the `Stmt` parameter of
    /// [`ReturningExpression`] for `INSERT` statements with this `Values`
    /// shape.
    type StmtKind;
}

impl<T, Tab> InsertStmtKind for crate::query_builder::ValuesClause<T, Tab> {
    type StmtKind = InsertStmt;
}

impl<V, Tab, QId, const STABLE_QUERY_ID: bool> InsertStmtKind
    for crate::query_builder::BatchInsert<V, Tab, QId, STABLE_QUERY_ID>
{
    type StmtKind = InsertStmt;
}

impl InsertStmtKind for crate::query_builder::insert_statement::DefaultValues {
    type StmtKind = InsertStmt;
}

impl<S, C> InsertStmtKind for crate::query_builder::insert_statement::InsertFromSelect<S, C> {
    type StmtKind = InsertStmt;
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
    type StmtKind = InsertStmt;
}

impl<V, Target, Changeset, Tab, WhereClause> InsertStmtKind
    for crate::query_builder::upsert::on_conflict_clause::OnConflictValues<
        V,
        Target,
        crate::query_builder::upsert::on_conflict_actions::DoUpdate<Changeset, Tab>,
        WhereClause,
    >
{
    type StmtKind = InsertOnConflictDoUpdateStmt;
}
