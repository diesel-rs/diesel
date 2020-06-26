//! AST types representing various typed SQL expressions.
//!
//! Almost all types implement either [`Expression`](trait.Expression.html) or
//! [`AsExpression`](trait.AsExpression.html).
//!
//! The most common expression to work with is a
//! [`Column`](../query_source/trait.Column.html). There are various methods
//! that you can call on these, found in
//! [`expression_methods`](../expression_methods).
//!
//! You can also use numeric operators such as `+` on expressions of the
//! appropriate type.
//!
//! Any primitive which implements [`ToSql`](../serialize/trait.ToSql.html) will
//! also implement [`AsExpression`](trait.AsExpression.html), allowing it to be
//! used as an argument to any of the methods described here.
#[macro_use]
#[doc(hidden)]
pub mod ops;
pub mod functions;

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
#[doc(hidden)]
pub mod grouped;
#[doc(hidden)]
pub mod helper_types;
mod not;
#[doc(hidden)]
pub mod nullable;
#[doc(hidden)]
#[macro_use]
pub mod operators;
#[doc(hidden)]
pub mod sql_literal;
#[doc(hidden)]
pub mod subselect;

#[doc(hidden)]
#[allow(non_camel_case_types)]
pub mod dsl {
    use crate::dsl::SqlTypeOf;

    #[doc(inline)]
    pub use super::count::*;
    #[doc(inline)]
    pub use super::exists::exists;
    #[doc(inline)]
    pub use super::functions::aggregate_folding::*;
    #[doc(inline)]
    pub use super::functions::aggregate_ordering::*;
    #[doc(inline)]
    pub use super::functions::date_and_time::*;
    #[doc(inline)]
    pub use super::not::not;
    #[doc(inline)]
    pub use super::sql_literal::sql;

    #[cfg(feature = "postgres")]
    pub use crate::pg::expression::dsl::*;

    /// The return type of [`count(expr)`](../dsl/fn.count.html)
    pub type count<Expr> = super::count::count::HelperType<SqlTypeOf<Expr>, Expr>;

    /// The return type of [`count_star()`](../dsl/fn.count_star.html)
    pub type count_star = super::count::CountStar;

    /// The return type of [`date(expr)`](../dsl/fn.date.html)
    pub type date<Expr> = super::functions::date_and_time::date::HelperType<Expr>;
}

#[doc(inline)]
pub use self::sql_literal::{SqlLiteral, UncheckedBind};

use crate::backend::Backend;
use crate::dsl::AsExprOf;
use crate::sql_types::{HasSqlType, SingleValue, SqlType};

/// Represents a typed fragment of SQL.
///
/// Apps should not need to implement this type directly, but it may be common
/// to use this in where clauses. Libraries should consider using
/// [`infix_operator!`](../macro.infix_operator.html) or
/// [`postfix_operator!`](../macro.postfix_operator.html) instead of
/// implementing this directly.
pub trait Expression {
    /// The type that this expression represents in SQL
    type SqlType: TypedExpressionType;
}

/// Marker trait for possible types of [`Expression::SqlType`]
///
/// [`Expression::SqlType`]: trait.Expression.html#associatedtype.SqlType
pub trait TypedExpressionType {}

/// Possible types for []`Expression::SqlType`]
///
/// [`Expression::SqlType`]: trait.Expression.html#associatedtype.SqlType
pub mod expression_types {
    use super::{QueryMetadata, TypedExpressionType};
    use crate::backend::Backend;
    use crate::sql_types::SingleValue;

    /// Query nodes with this expression type do not have a statically at compile
    /// time known expression type.
    ///
    /// An example for such a query node in diesel itself, is `sql_query` as
    /// we do not know which fields are returned from such a query at compile time.
    ///
    /// For loading values from queries returning a type of this expression, consider
    /// using [`#[derive(QueryableByName)]`] on the corresponding result type.
    ///
    /// [`#[derive(QueryableByName)]`]: ../deserialize/derive.QueryableByName.html
    #[derive(Clone, Copy, Debug)]
    pub struct Untyped;

    /// Query nodes witch cannot be part of a select clause.
    ///
    /// If you see an error message containing `FromSqlRow` and this type
    /// recheck that you have written a valid select clause
    #[derive(Debug, Clone, Copy)]
    pub struct NotSelectable;

    impl TypedExpressionType for Untyped {}
    impl TypedExpressionType for NotSelectable {}

    impl<ST> TypedExpressionType for ST where ST: SingleValue {}

    impl<DB: Backend> QueryMetadata<Untyped> for DB {
        fn row_metadata(_: &DB::MetadataLookup, row: &mut Vec<Option<DB::TypeMetadata>>) {
            row.push(None)
        }
    }
}

impl<T: Expression + ?Sized> Expression for Box<T> {
    type SqlType = T::SqlType;
}

impl<'a, T: Expression + ?Sized> Expression for &'a T {
    type SqlType = T::SqlType;
}

/// A helper to translate type level sql type information into
/// runtime type information for specific queries
///
/// If you do not implement a custom backend implementation
/// this trait is likely not relevant for you.
pub trait QueryMetadata<T>: Backend {
    /// The exact return value of this function is considerded to be a
    /// backend specific implementation detail. You should not rely on those
    /// values if you not own the corresponding backend
    fn row_metadata(lookup: &Self::MetadataLookup, out: &mut Vec<Option<Self::TypeMetadata>>);
}

impl<T, DB> QueryMetadata<T> for DB
where
    DB: Backend + HasSqlType<T>,
    T: SingleValue,
{
    fn row_metadata(lookup: &Self::MetadataLookup, out: &mut Vec<Option<Self::TypeMetadata>>) {
        out.push(Some(<DB as HasSqlType<T>>::metadata(lookup)))
    }
}

/// Converts a type to its representation for use in Diesel's query builder.
///
/// This trait is used directly. Apps should typically use [`IntoSql`] instead.
///
/// Implementations of this trait will generally do one of 3 things:
///
/// - Return `self` for types which are already parts of Diesel's query builder
/// - Perform some implicit coercion (for example, allowing [`now`] to be used as
///   both [`Timestamp`] and [`Timestamptz`].
/// - Indicate that the type has data which will be sent separately from the
///   query. This is generally referred as a "bind parameter". Types which
///   implement [`ToSql`] will generally implement `AsExpression` this way.
///
///   [`IntoSql`]: trait.IntoSql.html
///   [`now`]: ../dsl/struct.now.html
///   [`Timestamp`]: ../sql_types/struct.Timestamp.html
///   [`Timestamptz`]: ../pg/types/sql_types/struct.Timestamptz.html
///   [`ToSql`]: ../serialize/trait.ToSql.html
///
///  This trait could be [derived](derive.AsExpression.html)

pub trait AsExpression<T>
where
    T: SqlType + TypedExpressionType,
{
    /// The expression being returned
    type Expression: Expression<SqlType = T>;

    /// Perform the conversion
    fn as_expression(self) -> Self::Expression;
}

#[doc(inline)]
pub use diesel_derives::AsExpression;

impl<T, ST> AsExpression<ST> for T
where
    T: Expression<SqlType = ST>,
    ST: SqlType + TypedExpressionType,
{
    type Expression = Self;

    fn as_expression(self) -> Self {
        self
    }
}

/// Converts a type to its representation for use in Diesel's query builder.
///
/// This trait only exists to make usage of `AsExpression` more ergonomic when
/// the `SqlType` cannot be inferred. It is generally used when you need to use
/// a Rust value as the left hand side of an expression, or when you want to
/// select a constant value.
///
/// # Example
///
/// ```rust
/// # include!("../doctest_setup.rs");
/// # use schema::users;
/// #
/// # fn main() {
/// use diesel::sql_types::Text;
/// #   let conn = establish_connection();
/// let names = users::table
///     .select("The Amazing ".into_sql::<Text>().concat(users::name))
///     .load(&conn);
/// let expected_names = vec![
///     "The Amazing Sean".to_string(),
///     "The Amazing Tess".to_string(),
/// ];
/// assert_eq!(Ok(expected_names), names);
/// # }
/// ```
pub trait IntoSql {
    /// Convert `self` to an expression for Diesel's query builder.
    ///
    /// There is no difference in behavior between `x.into_sql::<Y>()` and
    /// `AsExpression::<Y>::as_expression(x)`.
    fn into_sql<T>(self) -> AsExprOf<Self, T>
    where
        Self: AsExpression<T> + Sized,
        T: SqlType + TypedExpressionType,
    {
        self.as_expression()
    }

    /// Convert `&self` to an expression for Diesel's query builder.
    ///
    /// There is no difference in behavior between `x.as_sql::<Y>()` and
    /// `AsExpression::<Y>::as_expression(&x)`.
    fn as_sql<'a, T>(&'a self) -> AsExprOf<&'a Self, T>
    where
        &'a Self: AsExpression<T>,
        T: SqlType + TypedExpressionType,
    {
        self.as_expression()
    }
}

impl<T> IntoSql for T {}

/// Indicates that all elements of an expression are valid given a from clause.
///
/// This is used to ensure that `users.filter(posts::id.eq(1))` fails to
/// compile. This constraint is only used in places where the nullability of a
/// SQL type doesn't matter (everything except `select` and `returning`). For
/// places where nullability is important, `SelectableExpression` is used
/// instead.
pub trait AppearsOnTable<QS: ?Sized>: Expression {}

impl<T: ?Sized, QS> AppearsOnTable<QS> for Box<T>
where
    T: AppearsOnTable<QS>,
    Box<T>: Expression,
{
}

impl<'a, T: ?Sized, QS> AppearsOnTable<QS> for &'a T
where
    T: AppearsOnTable<QS>,
    &'a T: Expression,
{
}

/// Indicates that an expression can be selected from a source.
///
/// Columns will implement this for their table. Certain special types, like
/// `CountStar` and `Bound` will implement this for all sources. Most compound
/// expressions will implement this if each of their parts implement it.
///
/// Notably, columns will not implement this trait for the right side of a left
/// join. To select a column or expression using a column from the right side of
/// a left join, you must call `.nullable()` on it.
pub trait SelectableExpression<QS: ?Sized>: AppearsOnTable<QS> {}

impl<T: ?Sized, QS> SelectableExpression<QS> for Box<T>
where
    T: SelectableExpression<QS>,
    Box<T>: AppearsOnTable<QS>,
{
}

impl<'a, T: ?Sized, QS> SelectableExpression<QS> for &'a T
where
    T: SelectableExpression<QS>,
    &'a T: AppearsOnTable<QS>,
{
}

/// Is this expression valid for a given group by clause?
///
/// Implementations of this trait must ensure that aggregate expressions are
/// not mixed with non-aggregate expressions.
///
/// For generic types, you can determine if your sub-expresssions can appear
/// together using the [`MixedAggregates`] trait.
///
/// `GroupByClause` will be a tuple containing the set of expressions appearing
/// in the `GROUP BY` portion of the query. If there is no `GROUP BY`, it will
/// be `()`.
///
/// This trait can be [derived]
///
/// [derived]: derive.ValidGrouping.html
/// [`MixedAggregates`]: trait.MixedAggregates.html
pub trait ValidGrouping<GroupByClause> {
    /// Is this expression aggregate?
    ///
    /// This type should always be one of the structs in the [`is_aggregate`]
    /// module. See the documentation of those structs for more details.
    ///
    /// [`is_aggregate`]: is_aggregate/index.html
    type IsAggregate;
}

impl<T: ValidGrouping<GB> + ?Sized, GB> ValidGrouping<GB> for Box<T> {
    type IsAggregate = T::IsAggregate;
}

impl<'a, T: ValidGrouping<GB> + ?Sized, GB> ValidGrouping<GB> for &'a T {
    type IsAggregate = T::IsAggregate;
}

#[doc(inline)]
pub use diesel_derives::ValidGrouping;

/// Can two `IsAggregate` types appear in the same expression?
///
/// You should never implement this trait. It will eventually become a trait
/// alias.
///
/// [`is_aggregate::Yes`] and [`is_aggregate::No`] can only appear with
/// themselves or [`is_aggregate::Never`]. [`is_aggregate::Never`] can appear
/// with anything.
///
/// [`is_aggregate::Yes`]: is_aggregate/struct.Yes.html
/// [`is_aggregate::No`]: is_aggregate/struct.No.html
/// [`is_aggregate::Never`]: is_aggregate/struct.Never.html
pub trait MixedAggregates<Other> {
    /// What is the resulting `IsAggregate` type?
    type Output;
}

#[allow(missing_debug_implementations, missing_copy_implementations)]
/// Possible values for `ValidGrouping::IsAggregate`
pub mod is_aggregate {
    use super::MixedAggregates;

    /// Yes, this expression is aggregate for the given group by clause.
    pub struct Yes;

    /// No, this expression is not aggregate with the given group by clause,
    /// but it might be aggregate with a different group by clause.
    pub struct No;

    /// This expression is never aggregate, and can appear with any other
    /// expression, regardless of whether it is aggregate.
    ///
    /// Examples of this are literals. `1` does not care about aggregation.
    /// `foo + 1` is always valid, regardless of whether `foo` appears in the
    /// group by clause or not.
    pub struct Never;

    impl MixedAggregates<Yes> for Yes {
        type Output = Yes;
    }

    impl MixedAggregates<Never> for Yes {
        type Output = Yes;
    }

    impl MixedAggregates<No> for No {
        type Output = No;
    }

    impl MixedAggregates<Never> for No {
        type Output = No;
    }

    impl<T> MixedAggregates<T> for Never {
        type Output = T;
    }
}

// Note that these docs are similar to but slightly different than the stable
// docs below. Make sure if you change these that you also change the docs
// below.
/// Trait alias to represent an expression that isn't aggregate by default.
///
/// This alias represents a type which is not aggregate if there is no group by
/// clause. More specifically, it represents for types which implement
/// [`ValidGrouping<()>`] where `IsAggregate` is [`is_aggregate::No`] or
/// [`is_aggregate::Yes`].
///
/// While this trait is a useful stand-in for common cases, `T: NonAggregate`
/// cannot always be used when `T: ValidGrouping<(), IsAggregate = No>` or
/// `T: ValidGrouping<(), IsAggregate = Never>` could be. For that reason,
/// unless you need to abstract over both columns and literals, you should
/// prefer to use [`ValidGrouping<()>`] in your bounds instead.
///
/// [`ValidGrouping<()>`]: trait.ValidGrouping.html
/// [`is_aggregate::Yes`]: is_aggregate/struct.Yes.html
/// [`is_aggregate::No`]: is_aggregate/struct.No.html
#[cfg(feature = "unstable")]
pub trait NonAggregate = ValidGrouping<()>
where
    <Self as ValidGrouping<()>>::IsAggregate:
        MixedAggregates<is_aggregate::No, Output = is_aggregate::No>;

// Note that these docs are similar to but slightly different than the unstable
// docs above. Make sure if you change these that you also change the docs
// above.
/// Trait alias to represent an expression that isn't aggregate by default.
///
/// This trait should never be implemented directly. It is replaced with a
/// trait alias when the `unstable` feature is enabled.
///
/// This alias represents a type which is not aggregate if there is no group by
/// clause. More specifically, it represents for types which implement
/// [`ValidGrouping<()>`] where `IsAggregate` is [`is_aggregate::No`] or
/// [`is_aggregate::Yes`].
///
/// While this trait is a useful stand-in for common cases, `T: NonAggregate`
/// cannot always be used when `T: ValidGrouping<(), IsAggregate = No>` or
/// `T: ValidGrouping<(), IsAggregate = Never>` could be. For that reason,
/// unless you need to abstract over both columns and literals, you should
/// prefer to use [`ValidGrouping<()>`] in your bounds instead.
///
/// [`ValidGrouping<()>`]: trait.ValidGrouping.html
/// [`is_aggregate::Yes`]: is_aggregate/struct.Yes.html
/// [`is_aggregate::No`]: is_aggregate/struct.No.html
#[cfg(not(feature = "unstable"))]
pub trait NonAggregate: ValidGrouping<()> {}

#[cfg(not(feature = "unstable"))]
impl<T> NonAggregate for T
where
    T: ValidGrouping<()>,
    T::IsAggregate: MixedAggregates<is_aggregate::No, Output = is_aggregate::No>,
{
}

use crate::query_builder::{QueryFragment, QueryId};

/// Helper trait used when boxing expressions.
///
/// In Rust you cannot create a trait object with more than one trait.
/// This type has all of the additional traits you would want when using
/// `Box<Expression>` as a single trait object.
///
/// This is typically used as the return type of a function.
/// For cases where you want to dynamically construct a query,
/// [boxing the query] is usually more ergonomic.
///
/// [boxing the query]: ../query_dsl/trait.QueryDsl.html#method.into_boxed
///
/// # Examples
///
/// ```rust
/// # include!("../doctest_setup.rs");
/// # use schema::users;
/// use diesel::sql_types::Bool;
///
/// # fn main() {
/// #     run_test().unwrap();
/// # }
/// #
/// # fn run_test() -> QueryResult<()> {
/// #     let conn = establish_connection();
/// enum Search {
///     Id(i32),
///     Name(String),
/// }
///
/// # /*
/// type DB = diesel::sqlite::Sqlite;
/// # */
///
/// fn find_user(search: Search) -> Box<dyn BoxableExpression<users::table, DB, SqlType = Bool>> {
///     match search {
///         Search::Id(id) => Box::new(users::id.eq(id)),
///         Search::Name(name) => Box::new(users::name.eq(name)),
///     }
/// }
///
/// let user_one = users::table
///     .filter(find_user(Search::Id(1)))
///     .first(&conn)?;
/// assert_eq!((1, String::from("Sean")), user_one);
///
/// let tess = users::table
///     .filter(find_user(Search::Name("Tess".into())))
///     .first(&conn)?;
/// assert_eq!((2, String::from("Tess")), tess);
/// #     Ok(())
/// # }
/// ```
pub trait BoxableExpression<QS, DB>
where
    DB: Backend,
    Self: Expression,
    Self: SelectableExpression<QS>,
    Self: ValidGrouping<(), IsAggregate = is_aggregate::No>,
    Self: QueryFragment<DB>,
{
}

impl<QS, T, DB> BoxableExpression<QS, DB> for T
where
    DB: Backend,
    T: Expression,
    T: SelectableExpression<QS>,
    T: ValidGrouping<(), IsAggregate = is_aggregate::No>,
    T: QueryFragment<DB>,
{
}

impl<'a, QS, ST, DB> QueryId for dyn BoxableExpression<QS, DB, SqlType = ST> + 'a {
    type QueryId = ();

    const HAS_STATIC_QUERY_ID: bool = false;
}

/// Converts a tuple of values into a tuple of Diesel expressions.
///
/// This trait is similar to [`AsExpression`], but it operates on tuples.
/// The expressions must all be of the same SQL type.
///
/// [`AsExpression`]: trait.AsExpression.html
pub trait AsExpressionList<ST> {
    /// The final output expression
    type Expression;

    /// Perform the conversion
    fn as_expression_list(self) -> Self::Expression;
}
