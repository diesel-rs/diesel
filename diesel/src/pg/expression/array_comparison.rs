use backend::*;
use expression::{AsExpression, Expression, NonAggregate};
use pg::{Pg, PgQueryBuilder};
use query_builder::*;
use query_builder::debug::DebugQueryBuilder;
use result::QueryResult;
use types::Array;

/// Creates a PostgreSQL `ANY` expression.
///
/// As with most bare functions, this is not exported by default. You can import
/// it specifically from `diesel::expression::any`, or glob import
/// `diesel::expression::dsl::*`
///
/// # Example
///
/// ```rust
/// # #[macro_use] extern crate diesel;
/// # include!("src/doctest_setup.rs");
/// # use diesel::expression::dsl::*;
/// #
/// # table! {
/// #     users {
/// #         id -> Serial,
/// #         name -> VarChar,
/// #     }
/// # }
/// #
/// # fn main() {
/// #     use self::users::dsl::*;
/// #     let connection = establish_connection();
/// #     connection.execute("INSERT INTO users (name) VALUES ('Jim')").unwrap();
/// let sean = (1, "Sean".to_string());
/// let jim = (3, "Jim".to_string());
/// let data = users.filter(name.eq(any(vec!["Sean", "Jim"])));
/// assert_eq!(Ok(vec![sean, jim]), data.load(&connection));
/// # }
/// ```
pub fn any<ST, T>(vals: T) -> Any<T::Expression> where
    T: AsExpression<Array<ST>>,
{
    Any::new(vals.as_expression())
}

/// Creates a PostgreSQL `ALL` expression.
///
/// As with most bare functions, this is not exported by default. You can import
/// it specifically from `diesel::expression::all`, or glob import
/// `diesel::expression::dsl::*`
///
/// # Example
///
/// ```rust
/// # #[macro_use] extern crate diesel;
/// # include!("src/doctest_setup.rs");
/// # use diesel::expression::dsl::*;
/// #
/// # table! {
/// #     users {
/// #         id -> Serial,
/// #         name -> VarChar,
/// #     }
/// # }
/// #
/// # fn main() {
/// #     use self::users::dsl::*;
/// #     let connection = establish_connection();
/// #     connection.execute("INSERT INTO users (name) VALUES ('Jim')").unwrap();
/// let tess = (2, "Tess".to_string());
/// let data = users.filter(name.ne(all(vec!["Sean", "Jim"])));
/// assert_eq!(Ok(vec![tess]), data.load(&connection));
/// # }
/// ```
pub fn all<ST, T>(vals: T) -> All<T::Expression> where
    T: AsExpression<Array<ST>>,
{
    All::new(vals.as_expression())
}

#[doc(hidden)]
#[derive(Debug, Copy, Clone)]
pub struct Any<Expr> {
    expr: Expr,
}

impl<Expr> Any<Expr> {
    fn new(expr: Expr) -> Self {
        Any {
            expr: expr,
        }
    }
}

impl<Expr, ST> Expression for Any<Expr> where
    Expr: Expression<SqlType=Array<ST>>,
{
    type SqlType = ST;
}

impl<Expr> QueryFragment<Pg> for Any<Expr> where
    Expr: QueryFragment<Pg>,
{
    fn to_sql(&self, out: &mut PgQueryBuilder) -> BuildQueryResult {
        out.push_sql("ANY(");
        try!(self.expr.to_sql(out));
        out.push_sql(")");
        Ok(())
    }

    fn collect_binds(&self, out: &mut <Pg as Backend>::BindCollector) -> QueryResult<()> {
        try!(self.expr.collect_binds(out));
        Ok(())
    }

    fn is_safe_to_cache_prepared(&self) -> bool {
        self.expr.is_safe_to_cache_prepared()
    }
}

impl<Expr> QueryFragment<Debug> for Any<Expr> where
    Expr: QueryFragment<Debug>,
{
    fn to_sql(&self, out: &mut DebugQueryBuilder) -> BuildQueryResult {
        out.push_sql("ANY(");
        try!(self.expr.to_sql(out));
        out.push_sql(")");
        Ok(())
    }

    fn collect_binds(&self, out: &mut <Debug as Backend>::BindCollector) -> QueryResult<()> {
        try!(self.expr.collect_binds(out));
        Ok(())
    }

    fn is_safe_to_cache_prepared(&self) -> bool {
        self.expr.is_safe_to_cache_prepared()
    }
}

impl_query_id!(Any<Expr>);
impl_selectable_expression!(Any<Expr>);

impl<Expr> NonAggregate for Any<Expr> where
    Expr: NonAggregate,
{
}

#[doc(hidden)]
#[derive(Debug, Copy, Clone)]
pub struct All<Expr> {
    expr: Expr,
}

impl<Expr> All<Expr> {
    fn new(expr: Expr) -> Self {
        All {
            expr: expr,
        }
    }
}

impl<Expr, ST> Expression for All<Expr> where
    Expr: Expression<SqlType=Array<ST>>,
{
    type SqlType = ST;
}

impl<Expr> QueryFragment<Pg> for All<Expr> where
    Expr: QueryFragment<Pg>,
{
    fn to_sql(&self, out: &mut PgQueryBuilder) -> BuildQueryResult {
        out.push_sql("ALL(");
        try!(self.expr.to_sql(out));
        out.push_sql(")");
        Ok(())
    }

    fn collect_binds(&self, out: &mut <Pg as Backend>::BindCollector) -> QueryResult<()> {
        try!(self.expr.collect_binds(out));
        Ok(())
    }

    fn is_safe_to_cache_prepared(&self) -> bool {
        self.expr.is_safe_to_cache_prepared()
    }
}

impl<Expr> QueryFragment<Debug> for All<Expr> where
    Expr: QueryFragment<Debug>,
{
    fn to_sql(&self, out: &mut DebugQueryBuilder) -> BuildQueryResult {
        out.push_sql("ALL(");
        try!(self.expr.to_sql(out));
        out.push_sql(")");
        Ok(())
    }

    fn collect_binds(&self, out: &mut <Debug as Backend>::BindCollector) -> QueryResult<()> {
        try!(self.expr.collect_binds(out));
        Ok(())
    }

    fn is_safe_to_cache_prepared(&self) -> bool {
        self.expr.is_safe_to_cache_prepared()
    }
}

impl_query_id!(All<Expr>);
impl_selectable_expression!(All<Expr>);

impl<Expr> NonAggregate for All<Expr> where
    Expr: NonAggregate,
{
}
