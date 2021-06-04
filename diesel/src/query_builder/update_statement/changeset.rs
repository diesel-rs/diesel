use crate::backend::Backend;
use crate::expression::grouped::Grouped;
use crate::expression::operators::Eq;
use crate::expression::AppearsOnTable;
use crate::query_builder::*;
use crate::query_source::{Column, QuerySource};
use crate::result::QueryResult;

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
    Left: Column,
    Right: AppearsOnTable<Left::Table>,
{
    type Target = Left::Table;
    type Changeset = Assign<Left, Right>;

    fn as_changeset(self) -> Self::Changeset {
        Assign {
            _column: self.left,
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
pub struct Assign<Col, Expr> {
    _column: Col,
    expr: Expr,
}

impl<T, U, DB> QueryFragment<DB> for Assign<T, U>
where
    DB: Backend,
    T: Column,
    U: QueryFragment<DB>,
{
    fn walk_ast(&self, mut out: AstPass<DB>) -> QueryResult<()> {
        out.push_identifier(T::NAME)?;
        out.push_sql(" = ");
        QueryFragment::walk_ast(&self.expr, out)
    }
}
