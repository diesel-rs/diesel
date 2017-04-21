//! AST types representing various typed SQL expressions. Almost all types
//! implement either [`Expression`](/diesel/expression/trait.Expression.html) or
//! [`AsExpression`](/diesel/expression/trait.AsExpression.html).
//!
//! The most common expression to work with is a
//! [`Column`](../query_source/trait.Column.html). There are various methods
//! that you can call on these, found in
//! [`expression_methods`](expression_methods/index.html). You can also call
//! numeric operators on types which have been passed to
//! [`operator_allowed!`](../macro.operator_allowed.html) or
//! [`numeric_expr!`](../macro.numeric_expr.html).
//!
//! Any primitive which implements [`ToSql`](../types/trait.ToSql.html) will
//! also implement [`AsExpression`](trait.AsExpression.html), allowing it to be
//! used as an argument to any of the methods described here.
#[macro_use]
#[doc(hidden)]
pub mod ops;

#[doc(hidden)]
pub mod array_comparison;
#[doc(hidden)]
pub mod bound;
#[doc(hidden)]
pub mod coerce;
#[doc(hidden)]
pub mod count;
#[doc(hidden)]
pub mod exists;
pub mod expression_methods;
#[doc(hidden)]
#[macro_use]
pub mod functions;
#[doc(hidden)]
pub mod grouped;
pub mod helper_types;
#[doc(hidden)]
pub mod nullable;
#[doc(hidden)]
#[macro_use]
pub mod predicates;
pub mod sql_literal;

/// Reexports various top level functions and core extensions that are too
/// generic to export by default. This module exists to conveniently glob import
/// in functions where you need them.
pub mod dsl {
    #[doc(inline)] pub use super::count::{count, count_star};
    #[doc(inline)] pub use super::functions::date_and_time::*;
    #[doc(inline)] pub use super::functions::aggregate_ordering::*;
    #[doc(inline)] pub use super::functions::aggregate_folding::*;
    #[doc(inline)] pub use super::sql_literal::sql;
    #[doc(inline)] pub use super::exists::exists;

    #[cfg(feature = "postgres")]
    pub use pg::expression::dsl::*;
}

pub use self::dsl::*;
pub use self::sql_literal::SqlLiteral;

use backend::Backend;

/// Represents a typed fragment of SQL. Apps should not need to implement this
/// type directly, but it may be common to use this as type boundaries.
/// Libraries should consider using
/// [`infix_predicate!`](../macro.infix_predicate.html) or
/// [`postfix_predicate!`](../macro.postfix_predicate.html) instead of
/// implementing this directly.
pub trait Expression {
    type SqlType;
}

impl<T: Expression + ?Sized> Expression for Box<T> {
    type SqlType = T::SqlType;
}

impl<'a, T: Expression + ?Sized> Expression for &'a T {
    type SqlType = T::SqlType;
}

/// Describes how a type can be represented as an expression for a given type.
/// These types couldn't just implement [`Expression`](trait.Expression.html)
/// directly, as many things can be used as an expression of multiple types.
/// (`String` for example, can be used as either
/// [`VarChar`](../types/type.VarChar.html) or
/// [`Text`](../types/struct.Text.html)).
///
/// This trait allows us to use primitives on the right hand side of various
/// expressions. For example `name.eq("Sean")`
pub trait AsExpression<T> {
    type Expression: Expression<SqlType=T>;

    fn as_expression(self) -> Self::Expression;
}

impl<T: Expression> AsExpression<T::SqlType> for T {
    type Expression = Self;

    fn as_expression(self) -> Self {
        self
    }
}

/// Indicates that all elements of an expression are valid given a from clause.
/// This is used to ensure that `users.filter(posts::id.eq(1))` fails to
/// compile. This constraint is only used in places where the nullability of a
/// SQL type doesn't matter (everything except `select` and `returning`). For
/// places where nullability is important, `SelectableExpression` is used
/// instead.
pub trait AppearsOnTable<QS: ?Sized>: Expression {
}

impl<T: ?Sized, QS> AppearsOnTable<QS> for Box<T> where
    T: AppearsOnTable<QS>,
    Box<T>: Expression,
{
}

impl<'a, T: ?Sized, QS> AppearsOnTable<QS> for &'a T where
    T: AppearsOnTable<QS>,
    &'a T: Expression,
{
}

/// Indicates that an expression can be selected from a source. Columns will
/// implement this for their table. Certain special types, like `CountStar` and
/// `Bound` will implement this for all sources. Most compound expressions will
/// implement this if each of their parts implement it.
///
/// Notably, columns will not implement this trait for the right side of a left
/// join. To select a column or expression using a column from the right side of
/// a left join, you must call `.nullable()` on it.
pub trait SelectableExpression<QS: ?Sized>: AppearsOnTable<QS> {
}

impl<T: ?Sized, QS> SelectableExpression<QS> for Box<T> where
    T: SelectableExpression<QS>,
    Box<T>: AppearsOnTable<QS>,
{
}

impl<'a, T: ?Sized, QS> SelectableExpression<QS> for &'a T where
    T: SelectableExpression<QS>,
    &'a T: AppearsOnTable<QS>,
{
}

/// Marker trait to indicate that an expression does not include any aggregate
/// functions. Used to ensure that aggregate expressions aren't mixed with
/// non-aggregate expressions in a select clause, and that they're never
/// included in a where clause.
pub trait NonAggregate {
}

impl<T: NonAggregate + ?Sized> NonAggregate for Box<T> {
}

impl<'a, T: NonAggregate + ?Sized> NonAggregate for &'a T {
}

use query_builder::{QueryFragment, QueryId};

/// Helper trait used when boxing expressions. This exists to work around the
/// fact that Rust will not let us use non-core types as bounds on a trait
/// object (you could not return `Box<Expression+NonAggregate>`)
pub trait BoxableExpression<QS, DB> where
    DB: Backend,
    Self: Expression,
    Self: SelectableExpression<QS>,
    Self: NonAggregate,
    Self: QueryFragment<DB>,
{}

impl<QS, T, DB> BoxableExpression<QS, DB> for T where
    DB: Backend,
    T: Expression,
    T: SelectableExpression<QS>,
    T: NonAggregate,
    T: QueryFragment<DB>,
{
}

impl<QS, ST, DB> QueryId for BoxableExpression<QS, DB, SqlType=ST> {
    type QueryId = ();

    fn has_static_query_id() -> bool {
        false
    }
}
