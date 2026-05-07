//! `RETURNING old.col` support for PostgreSQL 18 and later.

use crate::backend::{Backend, sql_dialect};
use crate::expression::{
    AppearsOnTable, Expression, SelectableExpression, ValidGrouping, is_aggregate,
};
use crate::query_builder::returning::returning_query_source::{
    ReturningQuerySource, UpdateStmt, ValidInReturningOf,
};
use crate::query_builder::{AstPass, QueryFragment, QueryId};
use crate::query_source::Column;
use crate::result::QueryResult;

/// Wraps a column to refer to its pre-modification value in the `RETURNING`
/// clause of a PostgreSQL `UPDATE` or `INSERT ... ON CONFLICT ... DO UPDATE`
/// statement.
///
/// This is the type returned by [`old`].
#[derive(Debug, Clone, Copy)]
pub struct Old<C>(C);

impl<C> Old<C> {
    pub(crate) fn new(c: C) -> Self {
        Old(c)
    }
}

/// Refer to the pre-modification value of `col` in a PostgreSQL `RETURNING`
/// clause.
///
/// This corresponds to the SQL `RETURNING old.col` syntax introduced in
/// PostgreSQL 18.
///
/// # Requires PostgreSQL 18 or newer
///
/// Diesel emits `old.col` in the SQL it sends to the database. Earlier
/// versions of PostgreSQL will reject the query at execution time.
///
/// # Statement compatibility
///
/// `old(col)` is valid inside the `RETURNING` clause of:
///
/// * an `UPDATE` statement, where it has the same Rust SQL type as `col`
///   (since every returned row necessarily came from a pre-existing row).
/// * an `INSERT ... ON CONFLICT ... DO UPDATE` statement, **but only when
///   wrapped in [`.nullable()`]**: rows that were freshly inserted (rather
///   than updated) have no `old` row, and `old.col` is `NULL` for them, so
///   for type-safe deserialization you must opt into a nullable Rust SQL
///   type. Writing `old(col)` directly (without `.nullable()`) in this
///   context is rejected at compile time.
///
/// Use of `old(col)` in plain `INSERT` (without `ON CONFLICT ... DO UPDATE`)
/// or `DELETE` `RETURNING` is rejected at compile time.
///
/// `Old<C>` is a regular [`Expression`] with `SqlType = C::SqlType`, so it
/// composes with the rest of Diesel's expression DSL — it can appear inside a
/// [`Selectable`] struct, be wrapped with [`.nullable()`] or
/// [`.assume_not_null()`], etc.
///
/// [`.nullable()`]: crate::ExpressionMethods::nullable
/// [`.assume_not_null()`]: crate::ExpressionMethods::assume_not_null
/// [`Expression`]: crate::expression::Expression
/// [`Selectable`]: crate::expression::Selectable
///
/// # Example
///
/// ```rust
/// # include!("../../doctest_setup.rs");
/// #
/// # #[cfg(feature = "postgres")]
/// # fn main() {
/// #     use schema::users::dsl::*;
/// #     use diesel::pg::returning::old;
/// #     let connection = &mut establish_connection();
/// let was_and_now = diesel::update(users.find(1))
///     .set(name.eq("Updated"))
///     .returning((old(name), name))
///     .get_result::<(String, String)>(connection);
/// assert_eq!(Ok(("Sean".to_string(), "Updated".to_string())), was_and_now);
/// # }
/// # #[cfg(not(feature = "postgres"))]
/// # fn main() {}
/// ```
pub fn old<C: Column>(col: C) -> Old<C> {
    Old::new(col)
}

impl<C> QueryId for Old<C> {
    type QueryId = ();

    const HAS_STATIC_QUERY_ID: bool = false;
}

impl<C> Expression for Old<C>
where
    C: Column + Expression,
{
    type SqlType = <C as Expression>::SqlType;
}

impl<C> ValidGrouping<()> for Old<C>
where
    C: Column,
{
    type IsAggregate = is_aggregate::No;
}

// The `SelectableExpression` impl is kept fully generic in `StmtKind` and
// `Table`; the actual restriction (`Old<C>` is only valid in `UPDATE`
// `RETURNING` — and, transitively via `Nullable`, in
// `INSERT ... ON CONFLICT ... DO UPDATE`) is expressed by the
// `Table: ValidInReturningOf<Old<C>, StmtKind>` where-clause. When the
// leaf isn't valid, the resulting error is anchored on
// `ValidInReturningOf`'s `#[diagnostic::on_unimplemented]`, which
// substitutes `{StmtKind}` and `{Self}` (the table) separately so they
// render in full rather than collapsing to `<..., ...>` in long
// `INSERT ... ON CONFLICT ...` chains.
//
// `AppearsOnTable` is intentionally generic in both `StmtKind` and
// `Table`: that's required so the `Nullable<Old<C>>` propagation
// (`Nullable<T>: AppearsOnTable<QS> where T: AppearsOnTable<QS>`) lights
// up for `ReturningQuerySource<InsertStmtWithOnConflictDoUpdate, _>` —
// the compile-time gate stays at `SelectableExpression`/`ValidInReturningOf`.
impl<C, StmtKind, Table> AppearsOnTable<ReturningQuerySource<StmtKind, Table>> for Old<C>
where
    C: Column,
    Self: Expression,
{
}

impl<C, StmtKind, Table> SelectableExpression<ReturningQuerySource<StmtKind, Table>> for Old<C>
where
    C: Column,
    Table: ValidInReturningOf<Self, StmtKind>,
    Self: AppearsOnTable<ReturningQuerySource<StmtKind, Table>>,
{
}

// Self of `ValidInReturningOf` is the table (mirrors the
// `AliasAppearsInFromClause` / `AliasAliasAppearsInFromClause` pattern):
// orphan-rule-friendly for downstream crates that define their own tables.
impl<C> ValidInReturningOf<Old<C>, UpdateStmt> for C::Table where C: Column {}

impl<C, DB> QueryFragment<DB> for Old<C>
where
    DB: Backend,
    Self: QueryFragment<DB, DB::ReturningClause>,
{
    fn walk_ast<'b>(&'b self, pass: AstPass<'_, 'b, DB>) -> QueryResult<()> {
        <Self as QueryFragment<DB, DB::ReturningClause>>::walk_ast(self, pass)
    }
}

impl<C, DB> QueryFragment<DB, sql_dialect::returning_clause::PgLikeReturningClause> for Old<C>
where
    DB: Backend<ReturningClause = sql_dialect::returning_clause::PgLikeReturningClause>,
    C: Column,
{
    fn walk_ast<'b>(&'b self, mut out: AstPass<'_, 'b, DB>) -> QueryResult<()> {
        out.push_sql("old.");
        out.push_identifier(C::NAME)?;
        Ok(())
    }
}
