//! This module contains the query dsl node definitions
//! for array comparison operations like `IN` and `NOT IN`

use super::expression_types::NotSelectable;
use crate::backend::{sql_dialect, Backend, SqlDialect};
use crate::expression::subselect::Subselect;
use crate::expression::{
    AppearsOnTable, AsExpression, Expression, SelectableExpression, TypedExpressionType,
    ValidGrouping,
};
use crate::query_builder::combination_clause::CombinationClause;
use crate::query_builder::{
    AstPass, BoxedSelectStatement, QueryFragment, QueryId, SelectQuery, SelectStatement,
};
use crate::result::QueryResult;
use crate::serialize::ToSql;
use crate::sql_types::{self, HasSqlType, SingleValue, SqlType};
use std::marker::PhantomData;

/// Query dsl node that represents a `left IN (values)`
/// expression
///
/// Third party backend can customize the [`QueryFragment`]
/// implementation of this query dsl node via
/// [`SqlDialect::ArrayComparison`]. A customized implementation
/// is expected to provide the same semantics as an ANSI SQL
/// `IN` expression.
///
/// The postgres backend provided a specialized implementation
/// by using `left = ANY(values)` as optimized variant instead
/// if this is possible. For cases where this is not possible
/// like for example if values is a vector of arrays we
/// generate an ordinary `IN` expression instead.
#[derive(Debug, Copy, Clone, QueryId, ValidGrouping)]
#[non_exhaustive]
pub struct In<T, U> {
    /// The expression on the left side of the `IN` keyword
    pub left: T,
    /// The values clause of the `IN` expression
    pub values: U,
}

/// Query dsl node that represents a `left NOT IN (values)`
/// expression
///
/// Third party backend can customize the [`QueryFragment`]
/// implementation of this query dsl node via
/// [`SqlDialect::ArrayComparison`]. A customized implementation
/// is expected to provide the same semantics as an ANSI SQL
/// `NOT IN` expression.0
///
/// The postgres backend provided a specialized implementation
/// by using `left != ALL(values)` as optimized variant instead
/// if this is possible. For cases where this is not possible
/// like for example if values is a vector of arrays we
/// generate a ordinary `NOT IN` expression instead
#[derive(Debug, Copy, Clone, QueryId, ValidGrouping)]
#[non_exhaustive]
pub struct NotIn<T, U> {
    /// The expression on the left side of the `NOT IN` keyword
    pub left: T,
    /// The values clause of the `NOT IN` expression
    pub values: U,
}

impl<T, U> In<T, U> {
    pub(crate) fn new(left: T, values: U) -> Self {
        In { left, values }
    }

    pub(crate) fn walk_ansi_ast<'b, DB>(&'b self, mut out: AstPass<'_, 'b, DB>) -> QueryResult<()>
    where
        DB: Backend,
        T: QueryFragment<DB>,
        U: QueryFragment<DB> + InExpression,
    {
        if self.values.is_empty() {
            out.push_sql("1=0");
        } else {
            self.left.walk_ast(out.reborrow())?;
            out.push_sql(" IN (");
            self.values.walk_ast(out.reborrow())?;
            out.push_sql(")");
        }
        Ok(())
    }
}

impl<T, U> NotIn<T, U> {
    pub(crate) fn new(left: T, values: U) -> Self {
        NotIn { left, values }
    }

    pub(crate) fn walk_ansi_ast<'b, DB>(&'b self, mut out: AstPass<'_, 'b, DB>) -> QueryResult<()>
    where
        DB: Backend,
        T: QueryFragment<DB>,
        U: QueryFragment<DB> + InExpression,
    {
        if self.values.is_empty() {
            out.push_sql("1=1");
        } else {
            self.left.walk_ast(out.reborrow())?;
            out.push_sql(" NOT IN (");
            self.values.walk_ast(out.reborrow())?;
            out.push_sql(")");
        }
        Ok(())
    }
}

impl<T, U> Expression for In<T, U>
where
    T: Expression,
    U: InExpression<SqlType = T::SqlType>,
    T::SqlType: SqlType,
    sql_types::is_nullable::IsSqlTypeNullable<T::SqlType>:
        sql_types::MaybeNullableType<sql_types::Bool>,
{
    type SqlType = sql_types::is_nullable::MaybeNullable<
        sql_types::is_nullable::IsSqlTypeNullable<T::SqlType>,
        sql_types::Bool,
    >;
}

impl<T, U> Expression for NotIn<T, U>
where
    T: Expression,
    U: InExpression<SqlType = T::SqlType>,
    T::SqlType: SqlType,
    sql_types::is_nullable::IsSqlTypeNullable<T::SqlType>:
        sql_types::MaybeNullableType<sql_types::Bool>,
{
    type SqlType = sql_types::is_nullable::MaybeNullable<
        sql_types::is_nullable::IsSqlTypeNullable<T::SqlType>,
        sql_types::Bool,
    >;
}

impl<T, U, DB> QueryFragment<DB> for In<T, U>
where
    DB: Backend,
    Self: QueryFragment<DB, DB::ArrayComparison>,
{
    fn walk_ast<'b>(&'b self, pass: AstPass<'_, 'b, DB>) -> QueryResult<()> {
        <Self as QueryFragment<DB, DB::ArrayComparison>>::walk_ast(self, pass)
    }
}

impl<T, U, DB> QueryFragment<DB, sql_dialect::array_comparison::AnsiSqlArrayComparison> for In<T, U>
where
    DB: Backend
        + SqlDialect<ArrayComparison = sql_dialect::array_comparison::AnsiSqlArrayComparison>,
    T: QueryFragment<DB>,
    U: QueryFragment<DB> + InExpression,
{
    fn walk_ast<'b>(&'b self, out: AstPass<'_, 'b, DB>) -> QueryResult<()> {
        self.walk_ansi_ast(out)
    }
}

impl<T, U, DB> QueryFragment<DB> for NotIn<T, U>
where
    DB: Backend,
    Self: QueryFragment<DB, DB::ArrayComparison>,
{
    fn walk_ast<'b>(&'b self, pass: AstPass<'_, 'b, DB>) -> QueryResult<()> {
        <Self as QueryFragment<DB, DB::ArrayComparison>>::walk_ast(self, pass)
    }
}

impl<T, U, DB> QueryFragment<DB, sql_dialect::array_comparison::AnsiSqlArrayComparison>
    for NotIn<T, U>
where
    DB: Backend
        + SqlDialect<ArrayComparison = sql_dialect::array_comparison::AnsiSqlArrayComparison>,
    T: QueryFragment<DB>,
    U: QueryFragment<DB> + InExpression,
{
    fn walk_ast<'b>(&'b self, out: AstPass<'_, 'b, DB>) -> QueryResult<()> {
        self.walk_ansi_ast(out)
    }
}

impl_selectable_expression!(In<T, U>);
impl_selectable_expression!(NotIn<T, U>);

/// This trait describes how a type is transformed to the
/// `IN (values)` value expression
///
/// Diesel provided several implementations here:
///
///  - An implementation for any [`Iterator`] over values
///    that implement [`AsExpression<ST>`] for the corresponding
///    sql type ST. The corresponding values clause will contain
///    bind statements for each individual value.
///  - An implementation for select statements, that returns
///    a single field. The corresponding values clause will contain
///    the sub query.
///
///  This trait is exposed for custom third party backends so
///  that they can restrict the [`QueryFragment`] implementations
///  for [`In`] and [`NotIn`].
pub trait AsInExpression<T: SqlType> {
    /// Type of the expression returned by [AsInExpression::as_in_expression]
    type InExpression: InExpression<SqlType = T>;

    /// Construct the diesel query dsl representation of
    /// the `IN (values)` clause for the given type
    #[allow(clippy::wrong_self_convention)]
    // That's a public api, we cannot just change it to
    // appease clippy
    fn as_in_expression(self) -> Self::InExpression;
}

impl<I, T, ST> AsInExpression<ST> for I
where
    I: IntoIterator<Item = T>,
    T: AsExpression<ST>,
    ST: SqlType + TypedExpressionType,
{
    type InExpression = Many<ST, T>;

    fn as_in_expression(self) -> Self::InExpression {
        Many {
            values: self.into_iter().collect(),
            p: PhantomData,
        }
    }
}

/// A marker trait that identifies query fragments that can be used in `IN(...)` and `NOT IN(...)`
/// clauses, (or `= ANY (...)` clauses on the Postgres backend)
///
/// These can be wrapped in [`In`] or [`NotIn`] query dsl nodes
pub trait InExpression {
    /// The SQL type of the inner values, which should be the same as the left of the `IN` or
    /// `NOT IN` clause
    type SqlType: SqlType;

    /// Returns `true` if self represents an empty collection
    /// Otherwise `false` is returned.
    fn is_empty(&self) -> bool;

    /// Returns `true` if the values clause represents
    /// bind values and each bind value is a postgres array type
    fn is_array(&self) -> bool;
}

impl<ST, F, S, D, W, O, LOf, G, H, LC> AsInExpression<ST>
    for SelectStatement<F, S, D, W, O, LOf, G, H, LC>
where
    ST: SqlType,
    Subselect<Self, ST>: Expression<SqlType = ST>,
    Self: SelectQuery<SqlType = ST>,
{
    type InExpression = Subselect<Self, ST>;

    fn as_in_expression(self) -> Self::InExpression {
        Subselect::new(self)
    }
}

impl<'a, ST, QS, DB, GB> AsInExpression<ST> for BoxedSelectStatement<'a, ST, QS, DB, GB>
where
    ST: SqlType,
    Subselect<BoxedSelectStatement<'a, ST, QS, DB, GB>, ST>: Expression<SqlType = ST>,
{
    type InExpression = Subselect<Self, ST>;

    fn as_in_expression(self) -> Self::InExpression {
        Subselect::new(self)
    }
}

impl<ST, Combinator, Rule, Source, Rhs> AsInExpression<ST>
    for CombinationClause<Combinator, Rule, Source, Rhs>
where
    ST: SqlType,
    Self: SelectQuery<SqlType = ST>,
    Subselect<Self, ST>: Expression<SqlType = ST>,
{
    type InExpression = Subselect<Self, ST>;

    fn as_in_expression(self) -> Self::InExpression {
        Subselect::new(self)
    }
}

/// Query dsl node for the `values` part of an `IN (values)` clause
/// containing a variable number of bind values.
///
/// Third party backend can customize the [`QueryFragment`]
/// implementation of this query dsl node via
/// [`SqlDialect::ArrayComparison`]. The default
/// implementation does generate one bind per value
/// in the `values` field.
///
/// Diesel provides an optimized implementation for Postgresql
/// like database systems that bind all values with one
/// bind value of the type `Array<ST>` instead.
#[derive(Debug, Clone)]
pub struct Many<ST, I> {
    /// The values contained in the `IN (values)` clause
    pub values: Vec<I>,
    p: PhantomData<ST>,
}

impl<ST, I, GB> ValidGrouping<GB> for Many<ST, I>
where
    ST: SingleValue,
    I: AsExpression<ST>,
    I::Expression: ValidGrouping<GB>,
{
    type IsAggregate = <I::Expression as ValidGrouping<GB>>::IsAggregate;
}

impl<ST, I> Expression for Many<ST, I>
where
    ST: TypedExpressionType,
{
    // Comma-ed fake expressions are not usable directly in SQL
    // This is only implemented so that we can use the usual SelectableExpression & co traits
    // as constraints for the same implementations on [`In`] and [`NotIn`]
    type SqlType = NotSelectable;
}

impl<ST, I> InExpression for Many<ST, I>
where
    ST: SqlType,
{
    type SqlType = ST;

    fn is_empty(&self) -> bool {
        self.values.is_empty()
    }

    fn is_array(&self) -> bool {
        ST::IS_ARRAY
    }
}

impl<ST, I, QS> SelectableExpression<QS> for Many<ST, I>
where
    Many<ST, I>: AppearsOnTable<QS>,
    ST: SingleValue,
    I: AsExpression<ST>,
    <I as AsExpression<ST>>::Expression: SelectableExpression<QS>,
{
}

impl<ST, I, QS> AppearsOnTable<QS> for Many<ST, I>
where
    Many<ST, I>: Expression,
    I: AsExpression<ST>,
    ST: SingleValue,
    <I as AsExpression<ST>>::Expression: SelectableExpression<QS>,
{
}

impl<ST, I, DB> QueryFragment<DB> for Many<ST, I>
where
    Self: QueryFragment<DB, DB::ArrayComparison>,
    DB: Backend,
{
    fn walk_ast<'b>(&'b self, pass: AstPass<'_, 'b, DB>) -> QueryResult<()> {
        <Self as QueryFragment<DB, DB::ArrayComparison>>::walk_ast(self, pass)
    }
}

impl<ST, I, DB> QueryFragment<DB, sql_dialect::array_comparison::AnsiSqlArrayComparison>
    for Many<ST, I>
where
    DB: Backend
        + HasSqlType<ST>
        + SqlDialect<ArrayComparison = sql_dialect::array_comparison::AnsiSqlArrayComparison>,
    ST: SingleValue,
    I: ToSql<ST, DB>,
{
    fn walk_ast<'b>(&'b self, out: AstPass<'_, 'b, DB>) -> QueryResult<()> {
        self.walk_ansi_ast(out)
    }
}

impl<ST, I> Many<ST, I> {
    pub(crate) fn walk_ansi_ast<'b, DB>(&'b self, mut out: AstPass<'_, 'b, DB>) -> QueryResult<()>
    where
        DB: Backend + HasSqlType<ST>,
        ST: SingleValue,
        I: ToSql<ST, DB>,
    {
        out.unsafe_to_cache_prepared();
        let mut first = true;
        for value in &self.values {
            if first {
                first = false;
            } else {
                out.push_sql(", ");
            }
            out.push_bind_param(value)?;
        }
        Ok(())
    }
}

impl<ST, I> QueryId for Many<ST, I> {
    type QueryId = ();

    const HAS_STATIC_QUERY_ID: bool = false;
}
