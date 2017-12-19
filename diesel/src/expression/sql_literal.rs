use std::marker::PhantomData;

use backend::Backend;
use expression::*;
use query_builder::*;
use query_dsl::RunQueryDsl;
use result::QueryResult;
use types::HasSqlType;

#[derive(Debug, Clone)]
/// Returned by the [`sql()`] function.
///
/// [`sql()`]: ../dsl/fn.sql.html
pub struct SqlLiteral<ST> {
    sql: String,
    _marker: PhantomData<ST>,
}

impl<ST> SqlLiteral<ST> {
    #[doc(hidden)]
    pub fn new(sql: String) -> Self {
        SqlLiteral {
            sql: sql,
            _marker: PhantomData,
        }
    }
}

impl<ST> Expression for SqlLiteral<ST> {
    type SqlType = ST;
}

impl<ST, DB> QueryFragment<DB> for SqlLiteral<ST>
where
    DB: Backend + HasSqlType<ST>,
{
    fn walk_ast(&self, mut out: AstPass<DB>) -> QueryResult<()> {
        out.unsafe_to_cache_prepared();
        out.push_sql(&self.sql);
        Ok(())
    }
}

impl_query_id!(noop: SqlLiteral<ST>);

impl<ST> Query for SqlLiteral<ST> {
    type SqlType = ST;
}

impl<ST, Conn> RunQueryDsl<Conn> for SqlLiteral<ST> {}

impl<QS, ST> SelectableExpression<QS> for SqlLiteral<ST> {}

impl<QS, ST> AppearsOnTable<QS> for SqlLiteral<ST> {}

impl<ST> NonAggregate for SqlLiteral<ST> {}

/// Use literal SQL in the query builder
///
/// Available for when you truly cannot represent something using the expression
/// DSL. You will need to provide the SQL type of the expression, in addition to
/// the SQL.
///
/// This function is intended for use when you need a small bit of raw SQL in
/// your query. If you want to write the entire query using raw SQL, use
/// [`sql_query`](../fn.sql_query.html) instead.
///
/// # Safety
///
/// The compiler will be unable to verify the correctness of the annotated type.
/// If you give the wrong type, it'll either return an error when deserializing
/// the query result or produce unexpected values.
///
/// # Examples
///
/// ```rust
/// # #[macro_use] extern crate diesel;
/// # include!("../doctest_setup.rs");
/// # fn main() {
/// #     run_test().unwrap();
/// # }
/// #
/// # fn run_test() -> QueryResult<()> {
/// #     use schema::users::dsl::*;
/// use diesel::dsl::sql;
/// #     let connection = establish_connection();
/// let user = users.filter(sql("name = 'Sean'")).first(&connection)?;
/// let expected = (1, String::from("Sean"));
/// assert_eq!(expected, user);
/// #     Ok(())
/// # }
/// ```
pub fn sql<ST>(sql: &str) -> SqlLiteral<ST> {
    SqlLiteral::new(sql.into())
}
