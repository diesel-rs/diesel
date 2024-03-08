use crate::backend::DieselReserveSpecialization;
use crate::expression::grouped::Grouped;
use crate::expression::operators::Eq;
use crate::expression::AppearsOnTable;
use crate::query_builder::*;
use crate::query_source::{Column, QuerySource};
use crate::Table;

/// Types which can be passed to
/// [`update.set`](UpdateStatement::set()).
///
/// This trait can be [derived](derive@AsChangeset)
pub trait AsChangeset {
    /// The table which `Self::Changeset` will be updating
    type Target: QuerySource;

    /// The update statement this type represents
    type Changeset;

    /// Convert `self` into the actual update statement being executed
    // This method is part of our public API
    // we won't change it to just appease clippy
    #[allow(clippy::wrong_self_convention)]
    fn as_changeset(self) -> Self::Changeset;
}

// This is a false positive, we reexport it later
#[allow(unreachable_pub)]
#[doc(inline)]
pub use diesel_derives::AsChangeset;

impl<T: AsChangeset> AsChangeset for Option<T> {
    type Target = T::Target;
    type Changeset = Option<T::Changeset>;

    fn as_changeset(self) -> Self::Changeset {
        self.map(AsChangeset::as_changeset)
    }
}

impl<Left, Right> AsChangeset for Eq<Left, Right>
where
    Left: AssignmentTarget,
    Right: AppearsOnTable<Left::Table>,
{
    type Target = Left::Table;
    type Changeset = Assign<<Left as AssignmentTarget>::QueryAstNode, Right>;

    fn as_changeset(self) -> Self::Changeset {
        Assign {
            target: self.left.into_target(),
            expr: self.right,
        }
    }
}

impl<Left, Right> AsChangeset for Grouped<Eq<Left, Right>>
where
    Eq<Left, Right>: AsChangeset,
{
    type Target = <Eq<Left, Right> as AsChangeset>::Target;

    type Changeset = <Eq<Left, Right> as AsChangeset>::Changeset;

    fn as_changeset(self) -> Self::Changeset {
        self.0.as_changeset()
    }
}

#[derive(Debug, Clone, Copy, QueryId)]
pub struct Assign<Target, Expr> {
    target: Target,
    expr: Expr,
}

impl<T, U, DB> QueryFragment<DB> for Assign<T, U>
where
    DB: Backend,
    T: QueryFragment<DB>,
    U: QueryFragment<DB>,
{
    fn walk_ast<'b>(&'b self, mut out: AstPass<'_, 'b, DB>) -> QueryResult<()> {
        QueryFragment::walk_ast(&self.target, out.reborrow())?;
        out.push_sql(" = ");
        QueryFragment::walk_ast(&self.expr, out.reborrow())
    }
}

/// Represents the left hand side of an assignment expression for an
/// assignment in [AsChangeset]. The vast majority of the time, this will
/// be a [Column]. However, in certain database backends, it's possible to
/// assign to an expression. For example, in Postgres, it's possible to
/// "UPDATE TABLE SET array_column\[1\] = 'foo'".
pub trait AssignmentTarget {
    /// Table the assignment is to
    type Table: Table;
    /// A wrapper around a type to assign to (this wrapper should implement
    /// [QueryFragment]).
    type QueryAstNode;

    /// Move this in to the AST node which should implement [QueryFragment].
    fn into_target(self) -> Self::QueryAstNode;
}

/// Represents a `Column` as an `AssignmentTarget`. The vast majority of
/// targets in an update statement will be `Column`s.
#[derive(Debug, Clone, Copy)]
pub struct ColumnWrapperForUpdate<C>(pub C);

impl<DB, C> QueryFragment<DB> for ColumnWrapperForUpdate<C>
where
    DB: Backend + DieselReserveSpecialization,
    C: Column,
{
    fn walk_ast<'b>(&'b self, mut out: AstPass<'_, 'b, DB>) -> QueryResult<()> {
        out.push_identifier(C::NAME)
    }
}

impl<C> AssignmentTarget for C
where
    C: Column,
{
    type Table = C::Table;
    type QueryAstNode = ColumnWrapperForUpdate<C>;

    fn into_target(self) -> Self::QueryAstNode {
        ColumnWrapperForUpdate(self)
    }
}
