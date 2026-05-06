//! `RETURNING old.col` support for PostgreSQL 18 and later.

use crate::backend::{Backend, sql_dialect};
use crate::expression::{Expression, ValidGrouping, is_aggregate};
use crate::query_builder::returning_clause::{ReturningExpression, UpdateStmt};
use crate::query_builder::{AstPass, QueryFragment, QueryId};
use crate::query_source::Column;
use crate::result::QueryResult;

/// Wraps a column to refer to its pre-modification value in the `RETURNING`
/// clause of a PostgreSQL `UPDATE` statement.
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
/// `old(col)` is currently only valid inside the `RETURNING` clause of an
/// `UPDATE` statement (use in `INSERT` or `DELETE` `RETURNING` is rejected at
/// compile time). Support for `INSERT ... ON CONFLICT DO UPDATE RETURNING
/// old.col` is planned as a follow-up.
///
/// # Example
///
/// ```rust,no_run
/// # include!("../../doctest_setup.rs");
/// #
/// # #[cfg(feature = "postgres")]
/// # fn main() {
/// #     use schema::users::dsl::*;
/// #     use diesel::pg::returning::old;
/// #     let connection = &mut establish_connection();
/// let (was, now): (String, String) = diesel::update(users.find(1))
///     .set(name.eq("Updated"))
///     .returning((old(name), name))
///     .get_result(connection)
///     .unwrap();
/// # let _ = (was, now);
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

impl<C> ReturningExpression<UpdateStmt, C::Table> for Old<C>
where
    C: Column,
{
    type SqlType = <C as Expression>::SqlType;
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
