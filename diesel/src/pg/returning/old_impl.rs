//! `RETURNING old.col` support for PostgreSQL 18 and later.

use crate::backend::{Backend, sql_dialect};
use crate::expression::{
    AppearsOnTable, Expression, SelectableExpression, ValidGrouping, is_aggregate,
};
use crate::query_builder::returning::{
    InsertStmtWithOnConflictDoUpdate, ReturningQuerySource, UpdateStmt,
};
use crate::query_builder::{AstPass, QueryFragment, QueryId};
use crate::query_source::{AppearsInFromClause, Column, Never, Once, QueryRelation};
use crate::result::QueryResult;

/// Wraps a column to refer to its pre-modification value in the `RETURNING`
/// clause of a PostgreSQL `UPDATE` or `INSERT ... ON CONFLICT ... DO UPDATE`
/// statement.
///
/// This is the type returned by [`old()`](old()).
#[derive(Debug, Clone, Copy)]
pub struct Old<C> {
    _column: C,
}

impl<C> Old<C> {
    pub(crate) fn new(c: C) -> Self {
        Old { _column: c }
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
/// or `DELETE` `RETURNING` is rejected at compile time, because it is not useful
/// there. (Note that `ON CONFLICT DO NOTHING` never returns untouched rows.)
///
/// [`.nullable()`]: crate::NullableExpressionMethods::nullable
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
/// #     // `RETURNING old.col` requires PostgreSQL 18+
/// #     let pg_version: i32 = diesel::dsl::sql::<diesel::sql_types::Integer>(
/// #         "SELECT current_setting('server_version_num')::int",
/// #     ).get_result(connection).unwrap();
/// #     if pg_version < 180000 { return; }
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

// `Old<C>` is selectable on a `RETURNING` clause whose statement-kind marker
// is `UpdateStmt`. It is deliberately *not* selectable on
// `ReturningQuerySource<InsertStmtWithOnConflictDoUpdate, _>` directly — only
// `Nullable<Old<C>>` is, via the existing `Nullable` machinery and the
// `ToInnerJoin` mapping in `returning_query_source` (which makes
// `InsertStmtWithOnConflictDoUpdate`'s inner-join "fall back" to `UpdateStmt`).
// That's how we force users to write `old(col).nullable()` in an
// `INSERT ... ON CONFLICT ... DO UPDATE RETURNING` and reject `old(col)`
// alone at compile time.
impl<C, QS> AppearsOnTable<QS> for Old<C>
where
    C: Column,
    Self: Expression,
    // Check that we have exactly one `old` identifier in the `RETURNING` clause.
    QS: AppearsInFromClause<OldIdent, Count = crate::query_source::Once>,
    // Check that the `old` identifier relates the table of that column.
    QS: AppearsInFromClause<
            ReturningQuerySource<OldIdent, C::Table>,
            Count = crate::query_source::Once,
        >,
{
}

/// Represents the identifier `old` in the `RETURNING` clause.
/// It is independent of the table of the column, and used as QS in marker in AppearsInFromClause.
///
/// We use this to typecheck that there is only one `old` identifier when we use `old`, so that
/// there is no ambiguity, and also as a generic
/// "any valid OLD statement-kind marker for ReturningQuerySource".
#[derive(Debug, Clone, Copy)]
pub struct OldIdent;

/// There is an `old.` in ReturningQuerySource<UpdateStmt, T>
///
/// Useful to check non-ambiguity of `old`
impl<StmtKind, T> AppearsInFromClause<OldIdent> for ReturningQuerySource<StmtKind, T> {
    type Count = Once;
}
/// There isn't one directly on tables
/// (this is useful for typechecking `old` in subqueries in returning)
impl<T> AppearsInFromClause<OldIdent> for T
where
    T: QueryRelation,
{
    type Count = Never;
}
/// There is an `old.` for T in ReturningQuerySource<UpdateStmt, T>
impl<T> AppearsInFromClause<ReturningQuerySource<OldIdent, T>>
    for ReturningQuerySource<UpdateStmt, T>
{
    type Count = Once;
}
/// There is an `old.` for T in ReturningQuerySource<InsertStmtWithOnConflictDoUpdate, T>
impl<T> AppearsInFromClause<ReturningQuerySource<OldIdent, T>>
    for ReturningQuerySource<InsertStmtWithOnConflictDoUpdate, T>
{
    type Count = Once;
}

// We intentionally did not add implementations for use of `old(col)` in plain `INSERT`
// (without `ON CONFLICT ... DO UPDATE`) or `DELETE` `RETURNING`, because it is not useful
// there as one can just use `RETURNING column_name`. (`ON CONFLICT DO NOTHING` never returns
// untouched rows, so allowing `ON CONFLICT DO NOTHING ... RETURNING` might be misleading to
// users on that regard - I, the author, have seen bugs caused by not being aware of this
// PG behavior.)

impl<C> SelectableExpression<ReturningQuerySource<UpdateStmt, C::Table>> for Old<C>
where
    C: Column,
    Self: AppearsOnTable<ReturningQuerySource<UpdateStmt, C::Table>>,
{
}

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

pub use return_type_helpers_reexported::*;

pub(crate) mod return_type_helpers_reexported {
    use super::Old;

    /// The return type of [`old(col)`](super::old()).
    #[allow(non_camel_case_types)]
    pub type old<C> = Old<C>;
}
