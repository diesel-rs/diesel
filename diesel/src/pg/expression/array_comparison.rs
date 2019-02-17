use expression::subselect::Subselect;
use expression::{AsExpression, Expression};
use pg::Pg;
use query_builder::*;
use result::QueryResult;
use sql_types::Array;

/// Creates a PostgreSQL `ANY` expression.
///
/// As with most bare functions, this is not exported by default. You can import
/// it specifically from `diesel::expression::any`, or glob import
/// `diesel::dsl::*`
///
/// # Example
///
/// ```rust
/// # #[macro_use] extern crate diesel;
/// # include!("../../doctest_setup.rs");
/// # use diesel::dsl::*;
/// #
/// # fn main() {
/// #     use schema::users::dsl::*;
/// #     let connection = establish_connection();
/// #     connection.execute("INSERT INTO users (name) VALUES ('Jim')").unwrap();
/// let sean = (1, "Sean".to_string());
/// let jim = (3, "Jim".to_string());
/// let data = users.filter(name.eq(any(vec!["Sean", "Jim"])));
/// assert_eq!(Ok(vec![sean, jim]), data.load(&connection));
/// # }
/// ```
pub fn any<ST, T>(vals: T) -> Any<T::Expression>
where
    T: AsArrayExpression<ST>,
{
    Any::new(vals.as_expression())
}

/// Creates a PostgreSQL `ALL` expression.
///
/// As with most bare functions, this is not exported by default. You can import
/// it specifically as `diesel::dsl::all`.
///
/// # Example
///
/// ```rust
/// # #[macro_use] extern crate diesel;
/// # include!("../../doctest_setup.rs");
/// # use diesel::dsl::*;
/// #
/// # fn main() {
/// #     use schema::users::dsl::*;
/// #     let connection = establish_connection();
/// #     connection.execute("INSERT INTO users (name) VALUES ('Jim')").unwrap();
/// let tess = (2, "Tess".to_string());
/// let data = users.filter(name.ne(all(vec!["Sean", "Jim"])));
/// assert_eq!(Ok(vec![tess]), data.load(&connection));
/// # }
/// ```
pub fn all<ST, T>(vals: T) -> All<T::Expression>
where
    T: AsArrayExpression<ST>,
{
    All::new(vals.as_expression())
}

#[doc(hidden)]
#[derive(Debug, Copy, Clone, QueryId, NonAggregate)]
pub struct Any<Expr> {
    expr: Expr,
}

impl<Expr> Any<Expr> {
    fn new(expr: Expr) -> Self {
        Any { expr: expr }
    }
}

impl<Expr, ST> Expression for Any<Expr>
where
    Expr: Expression<SqlType = Array<ST>>,
{
    type SqlType = ST;
}

impl<Expr> QueryFragment<Pg> for Any<Expr>
where
    Expr: QueryFragment<Pg>,
{
    fn walk_ast(&self, mut out: AstPass<Pg>) -> QueryResult<()> {
        out.push_sql("ANY(");
        self.expr.walk_ast(out.reborrow())?;
        out.push_sql(")");
        Ok(())
    }
}

impl_selectable_expression!(Any<Expr>);

#[doc(hidden)]
#[derive(Debug, Copy, Clone, QueryId, NonAggregate)]
pub struct All<Expr> {
    expr: Expr,
}

impl<Expr> All<Expr> {
    fn new(expr: Expr) -> Self {
        All { expr: expr }
    }
}

impl<Expr, ST> Expression for All<Expr>
where
    Expr: Expression<SqlType = Array<ST>>,
{
    type SqlType = ST;
}

impl<Expr> QueryFragment<Pg> for All<Expr>
where
    Expr: QueryFragment<Pg>,
{
    fn walk_ast(&self, mut out: AstPass<Pg>) -> QueryResult<()> {
        out.push_sql("ALL(");
        self.expr.walk_ast(out.reborrow())?;
        out.push_sql(")");
        Ok(())
    }
}

impl_selectable_expression!(All<Expr>);

pub trait AsArrayExpression<ST> {
    type Expression: Expression<SqlType = Array<ST>>;

    fn as_expression(self) -> Self::Expression;
}

impl<ST, T> AsArrayExpression<ST> for T
where
    T: AsExpression<Array<ST>>,
{
    type Expression = <T as AsExpression<Array<ST>>>::Expression;

    fn as_expression(self) -> Self::Expression {
        AsExpression::as_expression(self)
    }
}

impl<ST, S, F, W, O, L, Of, G, FU> AsArrayExpression<ST>
    for SelectStatement<S, F, W, O, L, Of, G, FU>
where
    Self: SelectQuery<SqlType = ST>,
{
    type Expression = Subselect<Self, Array<ST>>;

    fn as_expression(self) -> Self::Expression {
        Subselect::new(self)
    }
}

impl<'a, ST, QS, DB> AsArrayExpression<ST> for BoxedSelectStatement<'a, ST, QS, DB>
where
    Self: SelectQuery<SqlType = ST>,
{
    type Expression = Subselect<Self, Array<ST>>;

    fn as_expression(self) -> Self::Expression {
        Subselect::new(self)
    }
}
