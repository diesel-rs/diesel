use super::{SetClause, batch_update::*};
use crate::associations::HasTable;
use crate::backend::DieselReserveSpecialization;
use crate::expression::AppearsOnTable;
use crate::expression::grouped::Grouped;
use crate::expression::operators::Eq;
use crate::query_builder::*;
use crate::query_source::{Column, QuerySource};
use crate::{Identifiable, Table};
use alloc::borrow::ToOwned;

/// Types which can be passed to
/// [`update.set`](UpdateStatement::set()).
///
/// This trait can be [derived](derive@AsChangeset)
pub trait AsChangeset {
    /// The table which `Self::Changeset` will be updating
    type Target: QuerySource;

    /// The update statement this type represents
    type Changeset;

    /// Return the associated Update type. Defaults to Single row Updates.
    #[doc(hidden)]
    const SET_CLAUSE: SetClause = SetClause::Immediate;

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

impl<'update, T> AsChangeset for &'update Option<T>
where
    &'update T: AsChangeset,
{
    type Target = <&'update T as AsChangeset>::Target;
    type Changeset = Option<<&'update T as AsChangeset>::Changeset>;

    fn as_changeset(self) -> Self::Changeset {
        self.as_ref().map(AsChangeset::as_changeset)
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
    pub(crate) target: Target,
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

#[cfg(any(
    feature = "__sqlite-shared",
    feature = "postgres_backend",
    feature = "mysql_backend"
))]
impl<C, T, DB> BatchValueHelper<DB> for Assign<ColumnWrapperForUpdate<C>, T>
where
    DB: Backend + DieselReserveSpecialization,
    C: Column + QueryFragment<DB>,
    T: QueryFragment<DB>,
    Self: BatchAssignHelper<DB>,
{
    fn assign<'b>(&'b self, mut out: AstPass<'_, 'b, DB>) -> QueryResult<()> {
        self.batch_assign_identifier(out.reborrow())?;
        out.push_sql(" = ");
        out.push_identifier(BATCH_UPDATE_ALIAS)?;
        out.push_sql(".");
        out.push_identifier(C::NAME)?;
        Ok(())
    }

    fn column_name<'b>(&'b self, out: AstPass<'_, 'b, DB>) -> QueryResult<()> {
        self.target.walk_ast(out)
    }

    fn bind_value<'b>(&'b self, out: AstPass<'_, 'b, DB>) -> QueryResult<()> {
        self.expr.walk_ast(out)
    }
}

impl<'a, U, I, C, PK> AsChangeset for &'a [U]
where
    U: AsChangeset + HasTable<Table = U::Target>,
    U::Target: Table<PrimaryKey = PK>,
    &'a U: AsChangeset<Target = U::Target, Changeset = C> + Identifiable<Table = U::Target, Id = I>,
{
    type Target = U::Target;
    type Changeset = BatchUpdate<I, C, PK, U::Target>;
    const SET_CLAUSE: SetClause = SetClause::Delegated;

    fn as_changeset(self) -> Self::Changeset {
        let values = self
            .iter()
            .map(|value| (Identifiable::id(value), AsChangeset::as_changeset(value)))
            .collect::<Vec<_>>();
        BatchUpdate::new(values, U::table().primary_key())
    }
}

impl<'a, U> AsChangeset for &'a Vec<U>
where
    U: AsChangeset,
    &'a [U]: AsChangeset,
{
    type Target = U::Target;
    type Changeset = <&'a [U] as AsChangeset>::Changeset;
    const SET_CLAUSE: SetClause = <&'a [U] as AsChangeset>::SET_CLAUSE;

    fn as_changeset(self) -> Self::Changeset {
        (&**self).as_changeset()
    }
}

impl<'a, U, const N: usize> AsChangeset for &'a [U; N]
where
    &'a [U]: AsChangeset,
{
    type Target = <&'a [U] as AsChangeset>::Target;
    type Changeset = <&'a [U] as AsChangeset>::Changeset;
    const SET_CLAUSE: SetClause = <&'a [U] as AsChangeset>::SET_CLAUSE;

    fn as_changeset(self) -> Self::Changeset {
        self.as_slice().as_changeset()
    }
}

impl<U> AsChangeset for Vec<U>
where
    Box<[U]>: AsChangeset,
{
    type Target = <Box<[U]> as AsChangeset>::Target;
    type Changeset = <Box<[U]> as AsChangeset>::Changeset;
    const SET_CLAUSE: SetClause = <Box<[U]> as AsChangeset>::SET_CLAUSE;

    fn as_changeset(self) -> Self::Changeset {
        self.into_boxed_slice().as_changeset()
    }
}

impl<U, const N: usize> AsChangeset for Box<[U; N]>
where
    Box<[U]>: AsChangeset,
{
    type Target = <Box<[U]> as AsChangeset>::Target;
    type Changeset = <Box<[U]> as AsChangeset>::Changeset;
    const SET_CLAUSE: SetClause = <Box<[U]> as AsChangeset>::SET_CLAUSE;

    fn as_changeset(self) -> Self::Changeset {
        (self as Box<[U]>).as_changeset()
    }
}

impl<U, I, C, PK> AsChangeset for Box<[U]>
where
    U: AsChangeset<Changeset = C> + HasTable<Table = U::Target>,
    U::Target: Table<PrimaryKey = PK>,
    for<'a> &'a U: Identifiable<Id: IntoOwned<Owned = I>>,
{
    type Target = U::Target;
    type Changeset = BatchUpdate<I, C, PK, U::Target>;
    const SET_CLAUSE: SetClause = SetClause::Delegated;

    fn as_changeset(self) -> Self::Changeset {
        let values = self
            .into_iter()
            .map(|v| {
                // this clone is not that great, but we do not have
                // many other options. On the other hand the primary key
                // is often cheap to clone, especially compared
                // to sending an large set of updates to the DB, so it shouldn't
                // matter that much
                let id = v.id().into_owned();
                let changes = v.as_changeset();
                (id, changes)
            })
            .collect();
        BatchUpdate::new(values, U::table().primary_key())
    }
}

impl<U, I, C, PK, const N: usize> AsChangeset for [U; N]
where
    U: AsChangeset<Changeset = C> + HasTable<Table = U::Target>,
    U::Target: Table<PrimaryKey = PK>,
    for<'a> &'a U: Identifiable<Id: IntoOwned<Owned = I>>,
{
    type Target = U::Target;
    type Changeset = BatchUpdate<I, C, PK, U::Target>;
    const SET_CLAUSE: SetClause = SetClause::Delegated;

    fn as_changeset(self) -> Self::Changeset {
        let values = self
            .into_iter()
            .map(|v| {
                // this clone is not that great, but we do not have
                // many other options. On the other hand the primary key
                // is often cheap to clone, especially compared
                // to sending an large set of updates to the DB, so it shouldn't
                // matter that much
                let id = v.id().into_owned();
                let changes = v.as_changeset();
                (id, changes)
            })
            .collect();
        BatchUpdate::new(values, U::table().primary_key())
    }
}

pub(crate) trait IntoOwned {
    type Owned;

    fn into_owned(self) -> Self::Owned;
}

impl<T> IntoOwned for &T
where
    T: ToOwned,
{
    type Owned = T::Owned;

    fn into_owned(self) -> Self::Owned {
        (*self).to_owned()
    }
}
