//! `RETURNING old.col` support for PostgreSQL 18 and later.

use crate::backend::{Backend, sql_dialect};
use crate::expression::{Expression, ValidGrouping, is_aggregate};
use crate::query_builder::returning::returning_expression::{self, ReturningExpression};
use crate::query_builder::{AstPass, QueryFragment, QueryId};
use crate::query_source::Column;
use crate::result::QueryResult;
use crate::sql_types::IntoNullable;

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
/// * an `INSERT ... ON CONFLICT ... DO UPDATE` statement, where its Rust SQL
///   type is `Nullable<C::SqlType>` because rows that were freshly inserted
///   (rather than updated) have no `old` row, and `old.col` is `NULL` for
///   them.
///
/// Use of `old(col)` in plain `INSERT` or `DELETE` `RETURNING` is rejected at
/// compile time.
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

impl<C> ValidGrouping<()> for Old<C>
where
    C: Column,
{
    type IsAggregate = is_aggregate::No;
}

impl<C> ReturningExpression<returning_expression::UpdateStmt, C::Table> for Old<C>
where
    C: Column,
{
    type SqlType = <C as Expression>::SqlType;
}

impl<C> ReturningExpression<returning_expression::InsertOnConflictDoUpdateStmt, C::Table> for Old<C>
where
    C: Column,
    <C as Expression>::SqlType: IntoNullable,
{
    type SqlType = <<C as Expression>::SqlType as IntoNullable>::Nullable;
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
