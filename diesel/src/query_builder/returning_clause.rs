use super::{AstPass, DeleteStatement, InsertStatement, QueryFragment, UpdateStatement};
use crate::QuerySource;
use crate::backend::{Backend, DieselReserveSpecialization};
use crate::expression::{Expression, SelectableExpression};
use crate::query_builder::QueryId;
use crate::result::QueryResult;
use crate::sql_types::SingleValue;

#[derive(Debug, Clone, Copy, QueryId)]
pub struct NoReturningClause;

impl<DB> QueryFragment<DB> for NoReturningClause
where
    DB: Backend + DieselReserveSpecialization,
{
    fn walk_ast<'b>(&'b self, _: AstPass<'_, 'b, DB>) -> QueryResult<()> {
        Ok(())
    }
}

/// This type represents a SQL `Returning` clause
///
/// Custom backends can specialize the [`QueryFragment`]
/// implementation via
/// [`SqlDialect::ReturningClause`](crate::backend::SqlDialect::ReturningClause)
#[cfg_attr(
    feature = "i-implement-a-third-party-backend-and-opt-into-breaking-changes",
    cfg(feature = "i-implement-a-third-party-backend-and-opt-into-breaking-changes")
)]
#[derive(Debug, Clone, Copy, QueryId)]
pub struct ReturningClause<Expr>(pub Expr);

impl<Expr, DB> QueryFragment<DB> for ReturningClause<Expr>
where
    DB: Backend,
    Self: QueryFragment<DB, DB::ReturningClause>,
{
    fn walk_ast<'b>(&'b self, pass: AstPass<'_, 'b, DB>) -> QueryResult<()> {
        <Self as QueryFragment<DB, DB::ReturningClause>>::walk_ast(self, pass)
    }
}

impl<Expr, DB>
    QueryFragment<DB, crate::backend::sql_dialect::returning_clause::PgLikeReturningClause>
    for ReturningClause<Expr>
where
    DB: Backend<
        ReturningClause = crate::backend::sql_dialect::returning_clause::PgLikeReturningClause,
    >,
    Expr: QueryFragment<DB>,
{
    fn walk_ast<'b>(&'b self, mut out: AstPass<'_, 'b, DB>) -> QueryResult<()> {
        out.push_sql(" RETURNING ");
        self.0.walk_ast(out.reborrow())?;
        Ok(())
    }
}

pub trait ReturningClauseHelper<S> {
    type WithReturning;
}

impl<S, T, U, Op> ReturningClauseHelper<S> for InsertStatement<T, U, Op>
where
    T: QuerySource,
{
    type WithReturning = InsertStatement<T, U, Op, ReturningClause<S>>;
}

impl<S, T, U, V> ReturningClauseHelper<S> for UpdateStatement<T, U, V>
where
    T: QuerySource,
{
    type WithReturning = UpdateStatement<T, U, V, ReturningClause<S>>;
}

impl<S, T, U> ReturningClauseHelper<S> for DeleteStatement<T, U>
where
    T: QuerySource,
{
    type WithReturning = DeleteStatement<T, U, ReturningClause<S>>;
}

/// Statement-kind marker used as the `Stmt` parameter of
/// [`ReturningExpression`] for `INSERT` statements.
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
/// This is the only marker for which [`crate::pg::returning::Old`]
/// implements [`ReturningExpression`], which is what makes
/// `.returning(old(col))` compile-time exclusive to `UPDATE`.
#[allow(missing_copy_implementations)]
#[derive(Debug)]
pub struct UpdateStmt;

/// Statement-kind marker used as the `Stmt` parameter of
/// [`ReturningExpression`] for `DELETE` statements.
#[allow(missing_copy_implementations)]
#[derive(Debug)]
pub struct DeleteStmt;

/// Marks expressions that may appear in a `RETURNING` clause for a given
/// statement kind / target table combination.
///
/// This trait replaces a bare `SelectableExpression<Table>` bound on the
/// `RETURNING` lists of `INSERT`/`UPDATE`/`DELETE`. The extra `Stmt` parameter
/// lets the `SqlType` of an expression genuinely depend on which statement it
/// is being used inside — necessary because PostgreSQL's `RETURNING old.col`
/// produces different result types in different statement contexts.
///
/// In v1 (this crate), the only place where the `Stmt` parameter is observable
/// is `Old<C>: ReturningExpression<UpdateStmt, C::Table>`, which is the
/// mechanism by which `.returning(old(col))` is admitted in `UPDATE` and
/// rejected at compile time in `INSERT`/`DELETE`.
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
/// — including tuples mixing `Old<C>` with plain columns in the
/// `Stmt = UpdateStmt` case. The blanket impl is disambiguated from the tuple
/// impl using the `SingleValue` marker trait, which tuple SQL types never
/// implement.
///
/// [`SelectableExpression<Table>`]: crate::expression::SelectableExpression
#[diagnostic::on_unimplemented(
    message = "`{Self}` cannot appear in the `RETURNING` clause of this statement",
    note = "for `INSERT`/`UPDATE`/`DELETE` `RETURNING` lists, every element \n\
            must be a column, or a tuple of columns / `RETURNING`-valid \n\
            expressions, that belongs to the modified table.\n\
            `old(col)` is only valid in `UPDATE` `RETURNING` lists.",
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
