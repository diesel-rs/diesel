use super::{SetClause, batch_update::*};
use crate::associations::HasTable;
use crate::backend::DieselReserveSpecialization;
use crate::expression::bound::Bound;
use crate::expression::grouped::Grouped;
use crate::expression::operators::Eq;
use crate::expression::{AppearsOnTable, TypedExpressionType};
use crate::query_builder::*;
use crate::query_source::{Column, QuerySource};
use crate::serialize::{Output, ToSql};
use crate::sql_types::{HasSqlType, SqlType};
use crate::{Expression, Identifiable, Table};

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

    /// Return the associated Update type. Defaults to Single row Updates.
    fn set_clause() -> SetClause {
        SetClause::Immediate
    }
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

impl<C, T, Tab, DB> BatchColumn<Tab, DB> for Assign<ColumnWrapperForUpdate<C>, T>
where
    C: Column + BatchColumn<Tab, DB>,
    DB: Backend,
{
    type Table = Tab;

    fn walk_ast<'b>(&'b self, mut out: AstPass<'_, 'b, DB>) -> QueryResult<()> {
        self.target.0.walk_ast(out.reborrow())
    }
}

impl<C, T, Tab, DB> BatchColumnAssign<Tab, DB> for Assign<ColumnWrapperForUpdate<C>, T>
where
    C: Column + BatchColumnAssign<Tab, DB>,
    DB: Backend,
{
    type Table = Tab;

    fn walk_ast<'b>(
        &'b self,
        mut out: AstPass<'_, 'b, DB>,
        sep: &'_ str,
        ambiguous: bool,
        alias: &'_ str,
    ) -> QueryResult<()> {
        self.target
            .0
            .walk_ast(out.reborrow(), sep, ambiguous, alias)
    }
}

impl<C, T, ST, Tab, DB> BatchValue<ST, Tab, DB> for Assign<C, Bound<ST, T>>
where
    Bound<ST, T>: QueryFragment<DB>,
    DB: Backend + HasSqlType<ST>,
{
    type Table = Tab;
    type SqlType = ST;

    fn walk_ast<'b>(&'b self, mut out: AstPass<'_, 'b, DB>) -> QueryResult<()> {
        QueryFragment::walk_ast(&self.expr, out.reborrow())
    }
}

impl<C, T, ST> Expression for Assign<C, Bound<ST, T>>
where
    ST: SqlType + TypedExpressionType,
{
    type SqlType = ST;
}

impl<C, T, ST, DB> ToSql<ST, DB> for Assign<ColumnWrapperForUpdate<C>, Bound<ST, T>>
where
    C: alloc::fmt::Debug,
    T: ToSql<ST, DB>,
    ST: alloc::fmt::Debug + SqlType + TypedExpressionType,
    DB: Backend + HasSqlType<ST>,
{
    fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, DB>) -> crate::serialize::Result {
        self.expr.item.to_sql(out)
    }
}

// Identifiable takes ownership if not implemented on a reference.
// Following implementations:
// - impl<U> AsChangeset for Vec<U>
// - impl<U, const N: usize> AsChangeset for [U; N]
// - impl<U, const N: usize> AsChangeset for Box<[U; N]>
//
// result in compile error:
// the parameter type `U` may not live long enough
// ...so that the reference type `&'a U` does not outlive the data it points at
impl<'a, U, I, C, PK> AsChangeset for &'a [U]
where
    U: AsChangeset + HasTable<Table = U::Target>,
    U::Target: Table<PrimaryKey = PK>,
    &'a U: AsChangeset<Target = U::Target, Changeset = C> + Identifiable<Table = U::Target, Id = I>,
{
    type Target = U::Target;
    type Changeset = BatchUpdate<I, C, PK, U::Target, (), false>;

    fn as_changeset(self) -> Self::Changeset {
        let values = self
            .into_iter()
            .map(|value| (Identifiable::id(value), AsChangeset::as_changeset(value)))
            .collect::<Vec<_>>();
        BatchUpdate::new(values, U::table().primary_key())
    }

    fn set_clause() -> SetClause {
        SetClause::Delegated
    }
}

impl<'a, U> AsChangeset for &'a Vec<U>
where
    U: AsChangeset,
    &'a [U]: AsChangeset,
{
    type Target = U::Target;
    type Changeset = <&'a [U] as AsChangeset>::Changeset;

    fn as_changeset(self) -> Self::Changeset {
        (&**self).as_changeset()
    }

    fn set_clause() -> SetClause {
        SetClause::Delegated
    }
}

impl<'a, U, I, C, PK, const N: usize> AsChangeset for &'a [U; N]
where
    U: AsChangeset + HasTable<Table = U::Target>,
    U::Target: Table<PrimaryKey = PK>,
    &'a U: AsChangeset<Target = U::Target, Changeset = C> + Identifiable<Table = U::Target, Id = I>,
{
    type Target = U::Target;
    type Changeset = BatchUpdate<I, C, PK, U::Target, (), false>;

    fn as_changeset(self) -> Self::Changeset {
        let mut values = Vec::with_capacity(N);
        values.extend(
            self.into_iter()
                .map(|value| (Identifiable::id(value), AsChangeset::as_changeset(value))),
        );
        BatchUpdate::new(values, U::table().primary_key())
    }

    fn set_clause() -> SetClause {
        SetClause::Delegated
    }
}
