use crate::expression::subselect::Subselect;
use crate::expression::{AsExpression, Expression, ValidGrouping};
use crate::pg::Pg;
use crate::query_builder::*;
use crate::result::QueryResult;
use crate::sql_types::{Array, Bool};

#[cfg(all(feature = "with-deprecated", not(feature = "without-deprecated")))]
use crate::expression::TypedExpressionType;
#[cfg(all(feature = "with-deprecated", not(feature = "without-deprecated")))]
use crate::sql_types::SqlType;

#[cfg(all(feature = "with-deprecated", not(feature = "without-deprecated")))]
/// Creates a PostgreSQL `ANY` expression.
///
/// As with most bare functions, this is not exported by default. You can import
/// it specifically from `diesel::pg::expression::dsl::any`, or `diesel::dsl::any`.
///
/// # Example
///
/// ```rust
/// # include!("../../doctest_setup.rs");
/// # use diesel::dsl::*;
/// #
/// # fn main() {
/// #     use schema::users::dsl::*;
/// #     let connection = &mut establish_connection();
/// #     diesel::sql_query("INSERT INTO users (name) VALUES ('Jim')").execute(connection).unwrap();
/// let sean = (1, "Sean".to_string());
/// let jim = (3, "Jim".to_string());
/// let data = users.filter(name.eq(any(vec!["Sean", "Jim"])));
/// assert_eq!(Ok(vec![sean, jim]), data.load(connection));
/// # }
/// ```
#[deprecated(since = "2.0.0", note = "Use `ExpressionMethods::eq_any` instead")]
pub fn any<ST, T>(vals: T) -> Any<T::Expression>
where
    T: AsArrayExpression<ST>,
{
    Any::new(vals.as_expression())
}

#[cfg(all(feature = "with-deprecated", not(feature = "without-deprecated")))]
/// Creates a PostgreSQL `ALL` expression.
///
/// As with most bare functions, this is not exported by default. You can import
/// it specifically as `diesel::pg::expression::dsl::all`, or `diesel::dsl::all`.
///
/// # Example
///
/// ```rust
/// # include!("../../doctest_setup.rs");
/// # use diesel::dsl::*;
/// #
/// # fn main() {
/// #     use schema::users::dsl::*;
/// #     let connection = &mut establish_connection();
/// #     diesel::sql_query("INSERT INTO users (name) VALUES ('Jim')").execute(connection).unwrap();
/// let tess = (2, "Tess".to_string());
/// let data = users.filter(name.ne(all(vec!["Sean", "Jim"])));
/// assert_eq!(Ok(vec![tess]), data.load(connection));
/// # }
/// ```
#[deprecated(since = "2.0.0", note = "Use `ExpressionMethods::ne_all` instead")]
pub fn all<ST, T>(vals: T) -> All<T::Expression>
where
    T: AsArrayExpression<ST>,
{
    All::new(vals.as_expression())
}

#[cfg(all(feature = "with-deprecated", not(feature = "without-deprecated")))]
#[doc(hidden)]
#[derive(Debug, Copy, Clone, QueryId, ValidGrouping)]
pub struct Any<Expr> {
    expr: Expr,
}

#[cfg(all(feature = "with-deprecated", not(feature = "without-deprecated")))]
impl<Expr> Any<Expr> {
    fn new(expr: Expr) -> Self {
        Any { expr: expr }
    }
}

#[cfg(all(feature = "with-deprecated", not(feature = "without-deprecated")))]
impl<Expr, ST> Expression for Any<Expr>
where
    Expr: Expression<SqlType = Array<ST>>,
    ST: SqlType + TypedExpressionType,
{
    type SqlType = ST;
}

#[cfg(all(feature = "with-deprecated", not(feature = "without-deprecated")))]
impl<Expr> QueryFragment<Pg> for Any<Expr>
where
    Expr: QueryFragment<Pg>,
{
    fn walk_ast<'b>(&'b self, mut out: AstPass<'_, 'b, Pg>) -> QueryResult<()> {
        out.push_sql("ANY(");
        self.expr.walk_ast(out.reborrow())?;
        out.push_sql(")");
        Ok(())
    }
}

#[cfg(all(feature = "with-deprecated", not(feature = "without-deprecated")))]
impl_selectable_expression!(Any<Expr>);

#[cfg(all(feature = "with-deprecated", not(feature = "without-deprecated")))]
#[doc(hidden)]
#[derive(Debug, Copy, Clone, QueryId, ValidGrouping)]
pub struct All<Expr> {
    expr: Expr,
}

#[cfg(all(feature = "with-deprecated", not(feature = "without-deprecated")))]
impl<Expr> All<Expr> {
    fn new(expr: Expr) -> Self {
        All { expr: expr }
    }
}

#[cfg(all(feature = "with-deprecated", not(feature = "without-deprecated")))]
impl<Expr, ST> Expression for All<Expr>
where
    Expr: Expression<SqlType = Array<ST>>,
    ST: SqlType + TypedExpressionType,
{
    type SqlType = ST;
}

#[cfg(all(feature = "with-deprecated", not(feature = "without-deprecated")))]
impl<Expr> QueryFragment<Pg> for All<Expr>
where
    Expr: QueryFragment<Pg>,
{
    fn walk_ast<'b>(&'b self, mut out: AstPass<'_, 'b, Pg>) -> QueryResult<()> {
        out.push_sql("ALL(");
        self.expr.walk_ast(out.reborrow())?;
        out.push_sql(")");
        Ok(())
    }
}

#[cfg(all(feature = "with-deprecated", not(feature = "without-deprecated")))]
impl_selectable_expression!(All<Expr>);

/// Query dsl node for PostgreSQL `LIKE ANY(ARRAY[...])` expression
///
/// This allows matching a text expression against an array of patterns
/// using `LIKE` semantics (wildcard matching with `%` and `_`).
///
/// This is PostgreSQL-specific and not available on other backends.
///
/// # Example
///
/// ```rust
/// # include!("../../doctest_setup.rs");
/// # use diesel::dsl::*;
/// #
/// # fn main() {
/// #     use schema::users::dsl::*;
/// #     let connection = &mut establish_connection();
/// #     diesel::sql_query("INSERT INTO users (name) VALUES ('Jim')").execute(connection).unwrap();
/// let sean = (1, "Sean".to_string());
/// let jim = (3, "Jim".to_string());
/// let data = users.filter(name.like_any(vec!["Se%", "J%"]));
/// assert_eq!(Ok(vec![sean, jim]), data.load(connection));
/// # }
/// ```
#[derive(Debug, Copy, Clone, QueryId, ValidGrouping)]
#[non_exhaustive]
#[allow(unreachable_pub)]
pub struct LikeAny<T, U> {
    /// The expression on the left side of the `LIKE ANY` keyword
    pub left: T,
    /// The array of patterns to match against
    pub values: U,
}

impl<T, U> LikeAny<T, U> {
    pub(crate) fn new(left: T, values: U) -> Self {
        LikeAny { left, values }
    }
}

impl<T, U> Expression for LikeAny<T, U>
where
    T: Expression,
    T::SqlType: crate::sql_types::SqlType,
    U: Expression<SqlType = Array<T::SqlType>>,
    crate::sql_types::is_nullable::IsSqlTypeNullable<T::SqlType>:
        crate::sql_types::MaybeNullableType<Bool>,
{
    type SqlType = crate::sql_types::is_nullable::MaybeNullable<
        crate::sql_types::is_nullable::IsSqlTypeNullable<T::SqlType>,
        Bool,
    >;
}

impl<T, U> QueryFragment<Pg> for LikeAny<T, U>
where
    T: QueryFragment<Pg>,
    U: QueryFragment<Pg>,
{
    fn walk_ast<'b>(&'b self, mut out: AstPass<'_, 'b, Pg>) -> QueryResult<()> {
        self.left.walk_ast(out.reborrow())?;
        out.push_sql(" LIKE ANY(");
        self.values.walk_ast(out.reborrow())?;
        out.push_sql(")");
        Ok(())
    }
}

impl_selectable_expression!(LikeAny<T, U>);

/// Query dsl node for PostgreSQL `ILIKE ANY(ARRAY[...])` expression
///
/// This allows matching a text expression against an array of patterns
/// using case-insensitive `ILIKE` semantics.
///
/// This is PostgreSQL-specific and not available on other backends.
///
/// # Example
///
/// ```rust
/// # include!("../../doctest_setup.rs");
/// # use diesel::dsl::*;
/// #
/// # fn main() {
/// #     use schema::users::dsl::*;
/// #     let connection = &mut establish_connection();
/// #     diesel::sql_query("INSERT INTO users (name) VALUES ('Jim')").execute(connection).unwrap();
/// let sean = (1, "Sean".to_string());
/// let jim = (3, "Jim".to_string());
/// let data = users.filter(name.ilike_any(vec!["se%", "j%"]));
/// assert_eq!(Ok(vec![sean, jim]), data.load(connection));
/// # }
/// ```
#[derive(Debug, Copy, Clone, QueryId, ValidGrouping)]
#[non_exhaustive]
#[allow(unreachable_pub)]
pub struct ILikeAny<T, U> {
    /// The expression on the left side of the `ILIKE ANY` keyword
    pub left: T,
    /// The array of patterns to match against
    pub values: U,
}

impl<T, U> ILikeAny<T, U> {
    pub(crate) fn new(left: T, values: U) -> Self {
        ILikeAny { left, values }
    }
}

impl<T, U> Expression for ILikeAny<T, U>
where
    T: Expression,
    T::SqlType: crate::sql_types::SqlType,
    U: Expression<SqlType = Array<T::SqlType>>,
    crate::sql_types::is_nullable::IsSqlTypeNullable<T::SqlType>:
        crate::sql_types::MaybeNullableType<Bool>,
{
    type SqlType = crate::sql_types::is_nullable::MaybeNullable<
        crate::sql_types::is_nullable::IsSqlTypeNullable<T::SqlType>,
        Bool,
    >;
}

impl<T, U> QueryFragment<Pg> for ILikeAny<T, U>
where
    T: QueryFragment<Pg>,
    U: QueryFragment<Pg>,
{
    fn walk_ast<'b>(&'b self, mut out: AstPass<'_, 'b, Pg>) -> QueryResult<()> {
        self.left.walk_ast(out.reborrow())?;
        out.push_sql(" ILIKE ANY(");
        self.values.walk_ast(out.reborrow())?;
        out.push_sql(")");
        Ok(())
    }
}

impl_selectable_expression!(ILikeAny<T, U>);

/// Deprecated trait used for implementing `any` and `all` (which are themselves deprecated).
///
/// It has several quirks:
/// - `All<Expr: Expression<SqlType = Array<ST>>>` pretends to be an expression of type `ST`,
///   but it's in fact some custom that can really be only used in combination with the `=`
///   operator in some select places.
/// - Implementations that use Subelect below are lying: they pretend to be expressions of type
///   `Array<ST>`, but they're actually subselects, which are processed differently by Postgres
///   and may result in different query plans.
///   The `IntoArrayExpression` trait represents this more accurately, actually building an
///   actual expression of type array from a subselect (by wrapping it in `ARRAY(subselect)`).
pub trait AsArrayExpression<ST: 'static> {
    type Expression: Expression<SqlType = Array<ST>>;

    // This method is part of the public API
    // we won't change it to appease a clippy lint
    #[allow(clippy::wrong_self_convention)]
    fn as_expression(self) -> Self::Expression;
}

impl<ST, T> AsArrayExpression<ST> for T
where
    ST: 'static,
    T: AsExpression<Array<ST>>,
{
    type Expression = <T as AsExpression<Array<ST>>>::Expression;

    fn as_expression(self) -> Self::Expression {
        <T as AsExpression<Array<ST>>>::as_expression(self)
    }
}

impl<ST, F, S, D, W, O, LOf, G, H, LC> AsArrayExpression<ST>
    for SelectStatement<F, S, D, W, O, LOf, G, H, LC>
where
    ST: 'static,
    Self: SelectQuery<SqlType = ST>,
{
    type Expression = Subselect<Self, Array<ST>>;

    fn as_expression(self) -> Self::Expression {
        Subselect::new(self)
    }
}

impl<ST, QS, DB, GB> AsArrayExpression<ST> for BoxedSelectStatement<'_, ST, QS, DB, GB>
where
    ST: 'static,
    Self: SelectQuery<SqlType = ST>,
{
    type Expression = Subselect<Self, Array<ST>>;

    fn as_expression(self) -> Self::Expression {
        Subselect::new(self)
    }
}
