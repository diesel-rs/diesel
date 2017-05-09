use std::marker::PhantomData;

use backend::Backend;
use expression::*;
use query_builder::*;
use result::QueryResult;
use types::HasSqlType;

#[derive(Debug, Clone)]
/// Available for when you truly cannot represent something using the expression
/// DSL. You will need to provide the type of the expression, in addition to the
/// SQL. The compiler will be unable to verify the correctness of this type.
///
/// To get a SQL literal, use the [`sql()`] function.
///
/// [`sql()`]: fn.sql.html
pub struct SqlLiteral<ST> {
    sql: String,
    _marker: PhantomData<ST>,
}

impl<ST> SqlLiteral<ST> {
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

impl<ST, DB> QueryFragment<DB> for SqlLiteral<ST> where
    DB: Backend + HasSqlType<ST>,
{
    fn to_sql(&self, out: &mut DB::QueryBuilder) -> BuildQueryResult {
        out.push_sql(&self.sql);
        Ok(())
    }

    fn walk_ast(&self, pass: &mut AstPass<DB>) -> QueryResult<()> {
        if let AstPass::IsSafeToCachePrepared(ref mut result) = *pass {
            **result = false;
        }
        Ok(())
    }
}

impl_query_id!(noop: SqlLiteral<ST>);

impl<ST> Query for SqlLiteral<ST> {
    type SqlType = ST;
}

impl<QS, ST> SelectableExpression<QS> for SqlLiteral<ST> {
}

impl<QS, ST> AppearsOnTable<QS> for SqlLiteral<ST> {
}

impl<ST> NonAggregate for SqlLiteral<ST> {
}

/// Use literal SQL in the query builder
///
/// Available for when you truly cannot represent something using the expression
/// DSL. You will need to provide the SQL type of the expression, in addition to
/// the SQL.
///
/// # Safety
///
/// The compiler will be unable to verify the correctness of the annotated type.
/// If you give the wrong type, it'll either crash at runtime when deserializing
/// the query result or produce invalid values.
///
/// # Examples
///
/// ```rust
/// # #[macro_use] extern crate diesel;
/// # #[macro_use] extern crate diesel_codegen;
/// use diesel::expression::sql;
/// use diesel::types::{Bool, Integer, Text};
/// # include!("src/doctest_setup.rs");
/// # table! {
/// #   users {
/// #       id -> Integer,
/// #       name -> VarChar,
/// #   }
/// # }
///
/// #[derive(PartialEq, Debug, Queryable)]
/// struct User {
///     id: i32,
///     name: String,
/// }
///
/// # fn main() {
/// # let connection = establish_connection();
/// #
/// let setup = sql::<Bool>("INSERT INTO users(name) VALUES('Ruby')");
/// setup.execute(&connection).expect("Can't insert in users");
///
/// let query = sql::<(Integer, Text)>("SELECT id, name FROM users WHERE name='Ruby';");
/// let users = query.load::<User>(&connection).expect("Can't query users");
/// assert_eq!(users, vec![User{id: 3, name: "Ruby".to_owned()}]);
///
/// let query = users::table.filter(sql::<Bool>("name='Ruby'")); // Same query as above
/// let users = query.load::<User>(&connection).expect("Can't query users");
/// assert_eq!(users, vec![User{id: 3, name: "Ruby".to_owned()}]);
/// # }
/// ```
pub fn sql<ST>(sql: &str) -> SqlLiteral<ST> {
    SqlLiteral::new(sql.into())
}
