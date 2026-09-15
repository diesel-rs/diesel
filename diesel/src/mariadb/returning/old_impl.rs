//! `RETURNING old.col` support for PostgreSQL 18 and later.

use crate::backend::Backend;
use crate::expression::{
    AppearsOnTable, Expression, SelectableExpression, ValidGrouping, is_aggregate,
};
use crate::mariadb;
use crate::query_builder::returning::{OldIdent, ReturningQuerySource, UpdateStmt};
use crate::query_builder::{AstPass, QueryFragment, QueryId};
use crate::query_source::{AppearsInFromClause, Column};
use crate::result::QueryResult;

/// Wraps a column to refer to its pre-modification value in the `RETURNING`
/// clause of a Mariadb `UPDATE`  statement.
///
/// This is the type returned by [`old_value()`](old_value()).
#[derive(Debug, Clone, Copy)]
pub struct OldValue<C> {
    _column: C,
}

impl<C> OldValue<C> {
    pub(crate) fn new(c: C) -> Self {
        OldValue { _column: c }
    }
}

/// Refer to the pre-modification value of `col` in a Mariadb `RETURNING`
/// clause.
///
/// This corresponds to the SQL `RETURNING OLD_VALUE(col)` syntax introduced in
/// Mariadb 13.0.
///
/// # Requires Mariadb 13.0 or newer
///
/// Diesel emits `OLD_VALUE(col)` in the SQL it sends to the database. Earlier
/// versions of Mariadb will reject the query at execution time.
///
/// # Statement compatibility
///
/// `old_value(col)` is valid inside the `RETURNING` clause of:
///
/// * an `UPDATE` statement, where it has the same Rust SQL type as `col`
///   (since every returned row necessarily came from a pre-existing row).
///
/// Use of `old_value(col)` in `INSERT` or `DELETE` `RETURNING`
/// is rejected at compile time, because it is invalid
/// there. (Note that `ON CONFLICT DO NOTHING` never returns untouched rows.)
///
/// # Example
///
/// ```rust
/// # include!("../../doctest_setup.rs");
/// #
/// # #[cfg(feature = "mariadb")]
/// # fn main() {
/// #     use schema::users::dsl::*;
/// #     use diesel::mariadb::returning::old_value;
/// #     let connection = &mut establish_connection();
/// #     // `RETURNING OLD_VALUE(col)` requires Mariadb 13.0+
/// #     let mariadb_version = diesel::dsl::sql::<diesel::sql_types::VarChar>(
/// #         "SELECT VERSION()",
/// #     ).get_result::<String>(connection).unwrap()
/// #     .split('.')
/// #     .next().unwrap()
/// #     .parse::<u32>()
/// #     .unwrap();
/// #     if mariadb_version < 13 { return; }
/// let was_and_now = diesel::update(users.find(1))
///     .set(name.eq("Updated"))
///     .returning((old_value(name), name))
///     .get_result::<(String, String)>(connection);
/// assert_eq!(Ok(("Sean".to_string(), "Updated".to_string())), was_and_now);
/// # }
/// # #[cfg(not(feature = "mariadb"))]
/// # fn main() {}
/// ```
pub fn old_value<C: Column>(col: C) -> OldValue<C> {
    OldValue::new(col)
}

impl<C> QueryId for OldValue<C> {
    type QueryId = ();

    const HAS_STATIC_QUERY_ID: bool = false;
}

impl<C> Expression for OldValue<C>
where
    C: Column + Expression,
{
    type SqlType = <C as Expression>::SqlType;
}

impl<C> ValidGrouping<()> for OldValue<C>
where
    C: Column,
{
    type IsAggregate = is_aggregate::No;
}

// `OldValue<C>` is selectable on a `RETURNING` clause whose statement-kind marker
// is `UpdateStmt`. Since `OLD_VALUE` is only valid in `UPDATE ... RETURNING`
impl<C, QS> AppearsOnTable<QS> for OldValue<C>
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

// `old_value(col)` is only valid in `UPDATE ... RETURNING` for Mariadb,
// so we don't need to implement `SelectableExpression` for any other statement kinds.
impl<C> SelectableExpression<ReturningQuerySource<UpdateStmt, C::Table>> for OldValue<C>
where
    C: Column,
    Self: AppearsOnTable<ReturningQuerySource<UpdateStmt, C::Table>>,
{
}

impl<C, DB> QueryFragment<DB> for OldValue<C>
where
    DB: Backend,
    Self: QueryFragment<DB, DB::ReturningClause>,
{
    fn walk_ast<'b>(&'b self, pass: AstPass<'_, 'b, DB>) -> QueryResult<()> {
        <Self as QueryFragment<DB, DB::ReturningClause>>::walk_ast(self, pass)
    }
}

impl<C, DB> QueryFragment<DB, mariadb::backend::MariadbReturningClause> for OldValue<C>
where
    DB: Backend<ReturningClause = mariadb::backend::MariadbReturningClause>,
    C: Column,
{
    fn walk_ast<'b>(&'b self, mut out: AstPass<'_, 'b, DB>) -> QueryResult<()> {
        out.push_sql("OLD_VALUE(");
        out.push_identifier(C::NAME)?;
        out.push_sql(")");
        Ok(())
    }
}

pub use return_type_helpers_reexported::*;

pub(crate) mod return_type_helpers_reexported {
    use super::OldValue;

    /// The return type of [`old_value(col)`](super::old_value()).
    #[allow(non_camel_case_types)]
    pub type old_value<C> = OldValue<C>;
}
