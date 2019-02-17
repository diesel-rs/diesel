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
#[doc(hidden)]
#[macro_use]
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
    use dsl::SqlTypeOf;

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
    pub use pg::expression::dsl::*;

    /// The return type of [`count(expr)`](../dsl/fn.count.html)
    pub type count<Expr> = super::count::count::HelperType<SqlTypeOf<Expr>, Expr>;

    /// The return type of [`count_star)(`](../dsl/fn.count_star.html)
    pub type count_star = super::count::CountStar;

    /// The return type of [`date(expr)`](../dsl/fn.date.html)
    pub type date<Expr> = super::functions::date_and_time::date::HelperType<Expr>;
}

#[doc(inline)]
pub use self::sql_literal::{SqlLiteral, UncheckedBind};

use backend::Backend;
use dsl::AsExprOf;

/// Represents a typed fragment of SQL.
///
/// Apps should not need to implement this type directly, but it may be common
/// to use this in where clauses. Libraries should consider using
/// [`diesel_infix_operator!`](../macro.diesel_infix_operator.html) or
/// [`diesel_postfix_operator!`](../macro.diesel_postfix_operator.html) instead of
/// implementing this directly.
pub trait Expression {
    /// The type that this expression represents in SQL
    type SqlType;
}

impl<T: Expression + ?Sized> Expression for Box<T> {
    type SqlType = T::SqlType;
}

impl<'a, T: Expression + ?Sized> Expression for &'a T {
    type SqlType = T::SqlType;
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
/// ## Deriving
///
/// This trait can be automatically derived for any type which implements `ToSql`.
/// The type must be annotated with `#[sql_type = "SomeType"]`.
/// If that annotation appears multiple times,
/// implementations will be generated for each one of them.
///
/// This will generate the following impls:
///
/// - `impl AsExpression<SqlType> for YourType`
/// - `impl AsExpression<Nullable<SqlType>> for YourType`
/// - `impl AsExpression<SqlType> for &'a YourType`
/// - `impl AsExpression<Nullable<SqlType>> for &'a YourType`
/// - `impl AsExpression<SqlType> for &'a &'b YourType`
/// - `impl AsExpression<Nullable<SqlType>> for &'a &'b YourType`
///
/// If your type is unsized,
/// you can specify this by adding the annotation `#[diesel(not_sized)]`.
/// This will skip the impls for non-reference types.
pub trait AsExpression<T> {
    /// The expression being returned
    type Expression: Expression<SqlType = T>;

    /// Perform the conversion
    fn as_expression(self) -> Self::Expression;
}

impl<T: Expression> AsExpression<T::SqlType> for T {
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
/// # #[macro_use] extern crate diesel;
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

/// Marker trait to indicate that an expression does not include any aggregate
/// functions.
///
/// Used to ensure that aggregate expressions aren't mixed with
/// non-aggregate expressions in a select clause, and that they're never
/// included in a where clause.
///
/// ## Deriving
///
/// This trait can be automatically derived for structs with no type parameters
/// which are not aggregate, as well as for structs which are `NonAggregate`
/// when all type parameters are `NonAggregate`. For example:
///
/// ```ignore
/// #[derive(NonAggregate)]
/// struct Plus<Lhs, Rhs>(Lhs, Rhs);
///
/// // The following impl will be generated:
/// impl<Lhs, Rhs> NonAggregate for Plus<Lhs, Rhs>
/// where
///     Lhs: NonAggregate,
///     Rhs: NonAggregate,
/// {
/// }
/// ```
pub trait NonAggregate {}

impl<T: NonAggregate + ?Sized> NonAggregate for Box<T> {}

impl<'a, T: NonAggregate + ?Sized> NonAggregate for &'a T {}

use query_builder::{QueryFragment, QueryId};

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
/// # #[macro_use] extern crate diesel;
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
/// fn find_user(search: Search) -> Box<BoxableExpression<users::table, DB, SqlType = Bool>> {
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
    Self: NonAggregate,
    Self: QueryFragment<DB>,
{
}

impl<QS, T, DB> BoxableExpression<QS, DB> for T
where
    DB: Backend,
    T: Expression,
    T: SelectableExpression<QS>,
    T: NonAggregate,
    T: QueryFragment<DB>,
{
}

impl<'a, QS, ST, DB> QueryId for BoxableExpression<QS, DB, SqlType = ST> + 'a {
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
