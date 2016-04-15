use std::marker::PhantomData;

use backend::*;
use expression::{AsExpression, Expression, SelectableExpression, NonAggregate};
use pg::{Pg, PgQueryBuilder};
use query_builder::*;
use query_builder::debug::DebugQueryBuilder;
use result::QueryResult;
use types::{Array, HasSqlType};

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
pub fn any<ST, T>(vals: T) -> Any<T::Expression, ST> where
    Pg: HasSqlType<ST>,
    T: AsExpression<Array<ST>>,
{
    Any::new(vals.as_expression())
}

#[doc(hidden)]
pub struct Any<Expr, ST> {
    expr: Expr,
    _marker: PhantomData<ST>,
}

impl<Expr, ST> Any<Expr, ST> {
    fn new(expr: Expr) -> Self {
        Any {
            expr: expr,
            _marker: PhantomData,
        }
    }
}

impl<Expr, ST> Expression for Any<Expr, ST> where
    Pg: HasSqlType<ST>,
    Expr: Expression<SqlType=Array<ST>>,
{
    type SqlType = ST;
}

impl<Expr, ST> QueryFragment<Pg> for Any<Expr, ST> where
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
}

impl<Expr, ST> QueryFragment<Debug> for Any<Expr, ST> where
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
}

impl<Expr, ST, QS> SelectableExpression<QS> for Any<Expr, ST> where
    Pg: HasSqlType<ST>,
    Any<Expr, ST>: Expression,
    Expr: SelectableExpression<QS>,
{
}

impl<Expr, ST> NonAggregate for Any<Expr, ST> where
    Expr: NonAggregate,
    Any<Expr, ST>: Expression,
{
}
