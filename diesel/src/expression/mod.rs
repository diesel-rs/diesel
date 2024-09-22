//! AST types representing various typed SQL expressions.
//!
//! Almost all types implement either [`Expression`] or
//! [`AsExpression`].
//!
//! The most common expression to work with is a
//! [`Column`](crate::query_source::Column). There are various methods
//! that you can call on these, found in
//! [`expression_methods`](crate::expression_methods).
//!
//! You can also use numeric operators such as `+` on expressions of the
//! appropriate type.
//!
//! Any primitive which implements [`ToSql`](crate::serialize::ToSql) will
//! also implement [`AsExpression`], allowing it to be
//! used as an argument to any of the methods described here.
#[macro_use]
pub(crate) mod ops;
pub mod functions;

#[cfg(not(feature = "i-implement-a-third-party-backend-and-opt-into-breaking-changes"))]
pub(crate) mod array_comparison;
#[cfg(feature = "i-implement-a-third-party-backend-and-opt-into-breaking-changes")]
pub mod array_comparison;
pub(crate) mod assume_not_null;
pub(crate) mod bound;
mod coerce;
pub(crate) mod count;
#[cfg(not(feature = "i-implement-a-third-party-backend-and-opt-into-breaking-changes"))]
pub(crate) mod exists;
#[cfg(feature = "i-implement-a-third-party-backend-and-opt-into-breaking-changes")]
pub mod exists;
pub(crate) mod grouped;
pub(crate) mod helper_types;
mod not;
pub(crate) mod nullable;
#[macro_use]
pub(crate) mod operators;
mod case_when;
pub(crate) mod select_by;
mod sql_literal;
pub(crate) mod subselect;

#[cfg(feature = "i-implement-a-third-party-backend-and-opt-into-breaking-changes")]
pub use self::operators::Concat;

// we allow unreachable_pub here
// as rustc otherwise shows false positives
// for every item in this module. We reexport
// everything from `crate::helper_types::`
#[allow(non_camel_case_types, unreachable_pub)]
pub(crate) mod dsl {
    use crate::dsl::SqlTypeOf;

    #[doc(inline)]
    pub use super::case_when::case_when;
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
    pub use super::helper_types::{case_when, IntoSql, Otherwise, When};
    #[doc(inline)]
    pub use super::not::not;
    #[doc(inline)]
    pub use super::sql_literal::sql;

    #[cfg(feature = "postgres_backend")]
    pub use crate::pg::expression::dsl::*;

    /// The return type of [`count(expr)`](crate::dsl::count())
    pub type count<Expr> = super::count::count<SqlTypeOf<Expr>, Expr>;

    /// The return type of [`count_star()`](crate::dsl::count_star())
    pub type count_star = super::count::CountStar;

    /// The return type of [`count_distinct()`](crate::dsl::count_distinct())
    pub type count_distinct<Expr> = super::count::CountDistinct<SqlTypeOf<Expr>, Expr>;

    /// The return type of [`date(expr)`](crate::dsl::date())
    pub type date<Expr> = super::functions::date_and_time::date<Expr>;

    #[cfg(feature = "mysql_backend")]
    pub use crate::mysql::query_builder::DuplicatedKeys;
}

#[doc(inline)]
pub use self::case_when::CaseWhen;
#[doc(inline)]
pub use self::sql_literal::{SqlLiteral, UncheckedBind};

use crate::backend::Backend;
use crate::dsl::{AsExprOf, AsSelect};
use crate::sql_types::{HasSqlType, SingleValue, SqlType};

/// Represents a typed fragment of SQL.
///
/// Apps should not need to implement this type directly, but it may be common
/// to use this in where clauses. Libraries should consider using
/// [`infix_operator!`](crate::infix_operator!) or
/// [`postfix_operator!`](crate::postfix_operator!) instead of
/// implementing this directly.
pub trait Expression {
    /// The type that this expression represents in SQL
    type SqlType: TypedExpressionType;
}

/// Marker trait for possible types of [`Expression::SqlType`]
///
pub trait TypedExpressionType {}

/// Possible types for []`Expression::SqlType`]
///
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
    /// using [`#[derive(QueryableByName)]`](derive@crate::deserialize::QueryableByName)
    /// on the corresponding result type.
    ///
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
        fn row_metadata(_: &mut DB::MetadataLookup, row: &mut Vec<Option<DB::TypeMetadata>>) {
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
    /// The exact return value of this function is considered to be a
    /// backend specific implementation detail. You should not rely on those
    /// values if you not own the corresponding backend
    fn row_metadata(lookup: &mut Self::MetadataLookup, out: &mut Vec<Option<Self::TypeMetadata>>);
}

impl<T, DB> QueryMetadata<T> for DB
where
    DB: Backend + HasSqlType<T>,
    T: SingleValue,
{
    fn row_metadata(lookup: &mut Self::MetadataLookup, out: &mut Vec<Option<Self::TypeMetadata>>) {
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
///   [`IntoSql`]: crate::IntoSql
///   [`now`]: crate::dsl::now
///   [`Timestamp`]: crate::sql_types::Timestamp
///   [`Timestamptz`]: ../pg/types/sql_types/struct.Timestamptz.html
///   [`ToSql`]: crate::serialize::ToSql
///
///  This trait could be [derived](derive@AsExpression)
pub trait AsExpression<T>
where
    T: SqlType + TypedExpressionType,
{
    /// The expression being returned
    type Expression: Expression<SqlType = T>;

    /// Perform the conversion
    #[allow(clippy::wrong_self_convention)]
    // That's public API we cannot change it to appease clippy
    fn as_expression(self) -> Self::Expression;
}

#[doc(inline)]
pub use diesel_derives::AsExpression;

impl<T, ST> AsExpression<ST> for T
where
    T: Expression<SqlType = ST>,
    ST: SqlType + TypedExpressionType,
{
    type Expression = T;

    fn as_expression(self) -> T {
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
/// #   let conn = &mut establish_connection();
/// let names = users::table
///     .select("The Amazing ".into_sql::<Text>().concat(users::name))
///     .load(conn);
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
        <&'a Self as AsExpression<T>>::as_expression(self)
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
#[diagnostic::on_unimplemented(
    message = "Cannot select `{Self}` from `{QS}`",
    note = "`{Self}` is no valid selection for `{QS}`"
)]
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

/// Trait indicating that a record can be selected and queried from the database.
///
/// Types which implement `Selectable` represent the select clause of a SQL query.
/// Use [`SelectableHelper::as_select()`] to construct the select clause. Once you
/// called `.select(YourType::as_select())` we enforce at the type system level that you
/// use the same type to load the query result into.
///
/// The constructed select clause can contain arbitrary expressions coming from different
/// tables. The corresponding [derive](derive@Selectable) provides a simple way to
/// construct a select clause matching fields to the corresponding table columns.
///
/// # Examples
///
/// If you just want to construct a select clause using an existing struct, you can use
/// `#[derive(Selectable)]`, See [`#[derive(Selectable)]`](derive@Selectable) for details.
///
///
/// ```rust
/// # include!("../doctest_setup.rs");
/// #
/// use schema::users;
///
/// #[derive(Queryable, PartialEq, Debug, Selectable)]
/// struct User {
///     id: i32,
///     name: String,
/// }
///
/// # fn main() {
/// #     run_test();
/// # }
/// #
/// # fn run_test() -> QueryResult<()> {
/// #     use schema::users::dsl::*;
/// #     let connection = &mut establish_connection();
/// let first_user = users.select(User::as_select()).first(connection)?;
/// let expected = User { id: 1, name: "Sean".into() };
/// assert_eq!(expected, first_user);
/// #     Ok(())
/// # }
/// ```
///
/// Alternatively, we can implement the trait for our struct manually.
///
/// ```rust
/// # include!("../doctest_setup.rs");
/// #
/// use schema::users;
/// use diesel::prelude::{Queryable, Selectable};
/// use diesel::backend::Backend;
///
/// #[derive(Queryable, PartialEq, Debug)]
/// struct User {
///     id: i32,
///     name: String,
/// }
///
/// impl<DB> Selectable<DB> for User
/// where
///     DB: Backend
/// {
///     type SelectExpression = (users::id, users::name);
///
///     fn construct_selection() -> Self::SelectExpression {
///         (users::id, users::name)
///     }
/// }
///
/// # fn main() {
/// #     run_test();
/// # }
/// #
/// # fn run_test() -> QueryResult<()> {
/// #     use schema::users::dsl::*;
/// #     let connection = &mut establish_connection();
/// let first_user = users.select(User::as_select()).first(connection)?;
/// let expected = User { id: 1, name: "Sean".into() };
/// assert_eq!(expected, first_user);
/// #     Ok(())
/// # }
/// ```
///
/// When selecting from joined tables, you can select from a
/// composition of types that implement `Selectable`. The simplest way
/// is to use a tuple of all the types you wish to select.
///
/// ```rust
/// # include!("../doctest_setup.rs");
/// use schema::{users, posts};
///
/// #[derive(Debug, PartialEq, Queryable, Selectable)]
/// struct User {
///     id: i32,
///     name: String,
/// }
///
/// #[derive(Debug, PartialEq, Queryable, Selectable)]
/// struct Post {
///     id: i32,
///     user_id: i32,
///     title: String,
/// }
///
/// # fn main() -> QueryResult<()> {
/// #     let connection = &mut establish_connection();
/// #
/// let (first_user, first_post) = users::table
///     .inner_join(posts::table)
///     .select(<(User, Post)>::as_select())
///     .first(connection)?;
///
/// let expected_user = User { id: 1, name: "Sean".into() };
/// assert_eq!(expected_user, first_user);
///
/// let expected_post = Post { id: 1, user_id: 1, title: "My first post".into() };
/// assert_eq!(expected_post, first_post);
/// #
/// #     Ok(())
/// # }
/// ```
///
/// If you want to load only a subset of fields, you can create types
/// with those fields and use them in the composition.
///
/// ```rust
/// # include!("../doctest_setup.rs");
/// use schema::{users, posts};
///
/// #[derive(Debug, PartialEq, Queryable, Selectable)]
/// struct User {
///     id: i32,
///     name: String,
/// }
///
/// #[derive(Debug, PartialEq, Queryable, Selectable)]
/// #[diesel(table_name = posts)]
/// struct PostTitle {
///     title: String,
/// }
///
/// # fn main() -> QueryResult<()> {
/// #     let connection = &mut establish_connection();
/// #
/// let (first_user, first_post_title) = users::table
///     .inner_join(posts::table)
///     .select(<(User, PostTitle)>::as_select())
///     .first(connection)?;
///
/// let expected_user = User { id: 1, name: "Sean".into() };
/// assert_eq!(expected_user, first_user);
///
/// let expected_post_title = PostTitle { title: "My first post".into() };
/// assert_eq!(expected_post_title, first_post_title);
/// #
/// #     Ok(())
/// # }
/// ```
///
/// You are not limited to using only tuples to build the composed
/// type. The [`Selectable`](derive@Selectable) derive macro allows
/// you to *embed* other types. This is useful when you want to
/// implement methods or traits on the composed type.
///
/// ```rust
/// # include!("../doctest_setup.rs");
/// use schema::{users, posts};
///
/// #[derive(Debug, PartialEq, Queryable, Selectable)]
/// struct User {
///     id: i32,
///     name: String,
/// }
///
/// #[derive(Debug, PartialEq, Queryable, Selectable)]
/// #[diesel(table_name = posts)]
/// struct PostTitle {
///     title: String,
/// }
///
/// #[derive(Debug, PartialEq, Queryable, Selectable)]
/// struct UserPost {
///     #[diesel(embed)]
///     user: User,
///     #[diesel(embed)]
///     post_title: PostTitle,
/// }
///
/// # fn main() -> QueryResult<()> {
/// #     let connection = &mut establish_connection();
/// #
/// let first_user_post = users::table
///     .inner_join(posts::table)
///     .select(UserPost::as_select())
///     .first(connection)?;
///
/// let expected_user_post = UserPost {
///     user: User {
///         id: 1,
///         name: "Sean".into(),
///     },
///     post_title: PostTitle {
///         title: "My first post".into(),
///     },
/// };
/// assert_eq!(expected_user_post, first_user_post);
/// #
/// #     Ok(())
/// # }
/// ```
///
/// It is also possible to specify an entirely custom select expression
/// for fields when deriving [`Selectable`](derive@Selectable).
/// This is useful for example to
///
///  * avoid nesting types, or to
///  * populate fields with values other than table columns, such as
///    the result of an SQL function like `CURRENT_TIMESTAMP()`
///    or a custom SQL function.
///
/// The select expression is specified via the `select_expression` parameter.
///
/// Query fragments created using [`dsl::auto_type`](crate::dsl::auto_type) are supported, which
/// may be useful as the select expression gets large: it may not be practical to inline it in
/// the attribute then.
///
/// The type of the expression is usually inferred. If it can't be fully inferred automatically,
/// one may either:
/// - Put type annotations in inline blocks in the query fragment itself
/// - Use a dedicated [`dsl::auto_type`](crate::dsl::auto_type) function as `select_expression`
///   and use [`dsl::auto_type`'s type annotation features](crate::dsl::auto_type)
/// - Specify the type of the expression using the `select_expression_type` attribute
///
/// ```rust
/// # include!("../doctest_setup.rs");
/// use schema::{users, posts};
/// use diesel::dsl;
///
/// #[derive(Debug, PartialEq, Queryable, Selectable)]
/// struct User {
///     id: i32,
///     name: String,
/// }
///
/// #[derive(Debug, PartialEq, Queryable, Selectable)]
/// #[diesel(table_name = posts)]
/// struct PostTitle {
///     title: String,
/// }
///
/// #[derive(Debug, PartialEq, Queryable, Selectable)]
/// struct UserPost {
///     #[diesel(select_expression = users::columns::id)]
///     #[diesel(select_expression_type = users::columns::id)]
///     id: i32,
///     #[diesel(select_expression = users::columns::name)]
///     name: String,
///     #[diesel(select_expression = complex_fragment_for_title())]
///     title: String,
/// #   #[cfg(feature = "chrono")]
///     #[diesel(select_expression = diesel::dsl::now)]
///     access_time: chrono::NaiveDateTime,
///     #[diesel(select_expression = users::columns::id.eq({let id: i32 = FOO; id}))]
///     user_id_is_foo: bool,
/// }
/// const FOO: i32 = 42; // Type of FOO can't be inferred automatically in the select_expression
/// #[dsl::auto_type]
/// fn complex_fragment_for_title() -> _ {
///     // See the `#[dsl::auto_type]` documentation for examples of more complex usage
///     posts::columns::title
/// }
///
/// # fn main() -> QueryResult<()> {
/// #     let connection = &mut establish_connection();
/// #
/// let first_user_post = users::table
///     .inner_join(posts::table)
///     .select(UserPost::as_select())
///     .first(connection)?;
///
/// let expected_user_post = UserPost {
///     id: 1,
///     name: "Sean".into(),
///     title: "My first post".into(),
/// #   #[cfg(feature = "chrono")]
///     access_time: first_user_post.access_time,
///     user_id_is_foo: false,
/// };
/// assert_eq!(expected_user_post, first_user_post);
/// #
/// #     Ok(())
/// # }
/// ```
///
pub trait Selectable<DB: Backend> {
    /// The expression you'd like to select.
    ///
    /// This is typically a tuple of corresponding to the table columns of your struct's fields.
    type SelectExpression: Expression;

    /// Construct an instance of the expression
    fn construct_selection() -> Self::SelectExpression;
}

#[doc(inline)]
pub use diesel_derives::Selectable;

/// This helper trait provides several methods for
/// constructing a select or returning clause based on a
/// [`Selectable`] implementation.
pub trait SelectableHelper<DB: Backend>: Selectable<DB> + Sized {
    /// Construct a select clause based on a [`Selectable`] implementation.
    ///
    /// The returned select clause enforces that you use the same type
    /// for constructing the select clause and for loading the query result into.
    fn as_select() -> AsSelect<Self, DB>;

    /// An alias for `as_select` that can be used with returning clauses
    fn as_returning() -> AsSelect<Self, DB> {
        Self::as_select()
    }
}

impl<T, DB> SelectableHelper<DB> for T
where
    T: Selectable<DB>,
    DB: Backend,
{
    fn as_select() -> AsSelect<Self, DB> {
        select_by::SelectBy::new()
    }
}

/// Is this expression valid for a given group by clause?
///
/// Implementations of this trait must ensure that aggregate expressions are
/// not mixed with non-aggregate expressions.
///
/// For generic types, you can determine if your sub-expressions can appear
/// together using the [`MixedAggregates`] trait.
///
/// `GroupByClause` will be a tuple containing the set of expressions appearing
/// in the `GROUP BY` portion of the query. If there is no `GROUP BY`, it will
/// be `()`.
///
/// This trait can be [derived]
///
/// [derived]: derive@ValidGrouping
pub trait ValidGrouping<GroupByClause> {
    /// Is this expression aggregate?
    ///
    /// This type should always be one of the structs in the [`is_aggregate`]
    /// module. See the documentation of those structs for more details.
    ///
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

#[doc(hidden)]
pub trait IsContainedInGroupBy<T> {
    type Output;
}

#[doc(hidden)]
#[allow(missing_debug_implementations, missing_copy_implementations)]
pub mod is_contained_in_group_by {
    pub struct Yes;
    pub struct No;

    pub trait IsAny<O> {
        type Output;
    }

    impl<T> IsAny<T> for Yes {
        type Output = Yes;
    }

    impl IsAny<Yes> for No {
        type Output = Yes;
    }

    impl IsAny<No> for No {
        type Output = No;
    }
}

/// Can two `IsAggregate` types appear in the same expression?
///
/// You should never implement this trait. It will eventually become a trait
/// alias.
///
/// [`is_aggregate::Yes`] and [`is_aggregate::No`] can only appear with
/// themselves or [`is_aggregate::Never`]. [`is_aggregate::Never`] can appear
/// with anything.
///
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

#[cfg(feature = "unstable")]
// this needs to be a separate module for the reasons given in
// https://github.com/rust-lang/rust/issues/65860
mod unstable;

#[cfg(feature = "unstable")]
#[doc(inline)]
pub use self::unstable::NonAggregate;

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
/// [`ValidGrouping<()>`]: ValidGrouping
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
/// By default `BoxableExpression` is not usable in queries that have a custom
/// group by clause. Setting the generic parameters `GB` and `IsAggregate` allows
/// to configure the expression to be used with a specific group by clause.
///
/// This is typically used as the return type of a function.
/// For cases where you want to dynamically construct a query,
/// [boxing the query] is usually more ergonomic.
///
/// [boxing the query]: crate::query_dsl::QueryDsl::into_boxed()
///
/// # Examples
///
/// ## Usage without group by clause
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
/// #     let conn = &mut establish_connection();
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
///     .first(conn)?;
/// assert_eq!((1, String::from("Sean")), user_one);
///
/// let tess = users::table
///     .filter(find_user(Search::Name("Tess".into())))
///     .first(conn)?;
/// assert_eq!((2, String::from("Tess")), tess);
/// #     Ok(())
/// # }
/// ```
///
/// ## Allow usage with group by clause
///
/// ```rust
/// # include!("../doctest_setup.rs");
///
/// # use schema::users;
/// use diesel::sql_types::Text;
/// use diesel::dsl;
/// use diesel::expression::ValidGrouping;
///
/// # fn main() {
/// #     run_test().unwrap();
/// # }
/// #
/// # fn run_test() -> QueryResult<()> {
/// #     let conn = &mut establish_connection();
/// enum NameOrConst {
///     Name,
///     Const(String),
/// }
///
/// # /*
/// type DB = diesel::sqlite::Sqlite;
/// # */
///
/// fn selection<GB>(
///     selection: NameOrConst
/// ) -> Box<
///     dyn BoxableExpression<
///         users::table,
///         DB,
///         GB,
///         <users::name as ValidGrouping<GB>>::IsAggregate,
///         SqlType = Text
///     >
/// >
/// where
///     users::name: BoxableExpression<
///             users::table,
///             DB,
///             GB,
///             <users::name as ValidGrouping<GB>>::IsAggregate,
///             SqlType = Text
///         > + ValidGrouping<GB>,
/// {
///     match selection {
///         NameOrConst::Name => Box::new(users::name),
///         NameOrConst::Const(name) => Box::new(name.into_sql::<Text>()),
///     }
/// }
///
/// let user_one = users::table
///     .select(selection(NameOrConst::Name))
///     .first::<String>(conn)?;
/// assert_eq!(String::from("Sean"), user_one);
///
/// let with_name = users::table
///     .group_by(users::name)
///     .select(selection(NameOrConst::Const("Jane Doe".into())))
///     .first::<String>(conn)?;
/// assert_eq!(String::from("Jane Doe"), with_name);
/// #     Ok(())
/// # }
/// ```
///
/// ## More advanced query source
///
/// This example is a bit contrived, but in general, if you want to for example filter based on
/// different criteria on a joined table, you can use `InnerJoinQuerySource` and
/// `LeftJoinQuerySource` in the QS parameter of `BoxableExpression`.
///
/// ```rust
/// # include!("../doctest_setup.rs");
/// # use schema::{users, posts};
/// use diesel::sql_types::Bool;
/// use diesel::dsl::InnerJoinQuerySource;
///
/// # fn main() {
/// #     run_test().unwrap();
/// # }
/// #
/// # fn run_test() -> QueryResult<()> {
/// #     let conn = &mut establish_connection();
/// enum UserPostFilter {
///     User(i32),
///     Post(i32),
/// }
///
/// # /*
/// type DB = diesel::sqlite::Sqlite;
/// # */
///
/// fn filter_user_posts(
///     filter: UserPostFilter,
/// ) -> Box<dyn BoxableExpression<InnerJoinQuerySource<users::table, posts::table>, DB, SqlType = Bool>>
/// {
///     match filter {
///         UserPostFilter::User(user_id) => Box::new(users::id.eq(user_id)),
///         UserPostFilter::Post(post_id) => Box::new(posts::id.eq(post_id)),
///     }
/// }
///
/// let post_by_user_one = users::table
///     .inner_join(posts::table)
///     .filter(filter_user_posts(UserPostFilter::User(2)))
///     .select((posts::title, users::name))
///     .first::<(String, String)>(conn)?;
///
/// assert_eq!(
///     ("My first post too".to_string(), "Tess".to_string()),
///     post_by_user_one
/// );
/// #     Ok(())
/// # }
/// ```
pub trait BoxableExpression<QS, DB, GB = (), IsAggregate = is_aggregate::No>
where
    DB: Backend,
    Self: Expression,
    Self: SelectableExpression<QS>,
    Self: QueryFragment<DB>,
    Self: Send,
{
}

impl<QS, T, DB, GB, IsAggregate> BoxableExpression<QS, DB, GB, IsAggregate> for T
where
    DB: Backend,
    T: Expression,
    T: SelectableExpression<QS>,
    T: ValidGrouping<GB>,
    T: QueryFragment<DB>,
    T: Send,
    T::IsAggregate: MixedAggregates<IsAggregate, Output = IsAggregate>,
{
}

impl<'a, QS, ST, DB, GB, IsAggregate> QueryId
    for dyn BoxableExpression<QS, DB, GB, IsAggregate, SqlType = ST> + 'a
{
    type QueryId = ();

    const HAS_STATIC_QUERY_ID: bool = false;
}

impl<'a, QS, ST, DB, GB, IsAggregate> ValidGrouping<GB>
    for dyn BoxableExpression<QS, DB, GB, IsAggregate, SqlType = ST> + 'a
{
    type IsAggregate = IsAggregate;
}

/// Converts a tuple of values into a tuple of Diesel expressions.
///
/// This trait is similar to [`AsExpression`], but it operates on tuples.
/// The expressions must all be of the same SQL type.
///
pub trait AsExpressionList<ST> {
    /// The final output expression
    type Expression;

    /// Perform the conversion
    // That's public API, we cannot change
    // that to appease clippy
    #[allow(clippy::wrong_self_convention)]
    fn as_expression_list(self) -> Self::Expression;
}
