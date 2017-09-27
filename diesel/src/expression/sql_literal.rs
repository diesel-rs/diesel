use std::marker::PhantomData;

use backend::Backend;
use expression::*;
use query_builder::*;
use result::QueryResult;
#[cfg(feature = "with-deprecated")]
use super::unchecked_bind::UncheckedBind;
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
    #[doc(hidden)]
    pub fn new(sql: String) -> Self {
        SqlLiteral {
            sql: sql,
            _marker: PhantomData,
        }
    }

    /// Bind a value for use with this SQL query.
    ///
    /// # Safety
    ///
    /// This function should be used with care, as Diesel cannot validate that
    /// the value is of the right type nor can it validate that you have passed
    /// the correct number of parameters.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # #[macro_use] extern crate diesel;
    /// # include!("../doctest_setup.rs");
    /// #
    /// # table! {
    /// #     users {
    /// #         id -> Integer,
    /// #         name -> VarChar,
    /// #     }
    /// # }
    /// #
    /// # fn main() {
    /// #     use self::users::dsl::*;
    /// #     use diesel::dsl::sql;
    /// #     use diesel::types::{Integer, Text};
    /// #     let connection = establish_connection();
    /// #[cfg(feature="postgres")]
    /// let query = sql::<Integer>("SELECT id FROM users WHERE name = $1");
    /// #[cfg(not(feature="postgres"))]
    /// let query = sql::<Integer>("SELECT id FROM users WHERE name = ?");
    /// let seans_id = query.clone().bind::<Text, _>("Sean")
    ///     .get_result(&connection);
    /// assert_eq!(Ok(1), seans_id);
    /// let tess_id = query.bind::<Text, _>("Tess")
    ///     .get_result(&connection);
    /// assert_eq!(Ok(2), tess_id);
    /// # }
    /// ```
    ///
    /// ### Multiple Bind Params
    ///
    /// ```rust
    /// # #[macro_use] extern crate diesel;
    /// # include!("../doctest_setup.rs");
    /// #
    /// # table! {
    /// #     users {
    /// #         id -> Integer,
    /// #         name -> VarChar,
    /// #     }
    /// # }
    /// #
    /// # fn main() {
    /// #     use self::users::dsl::*;
    /// #     use diesel::dsl::sql;
    /// #     use diesel::types::{Integer, Text};
    /// #     let connection = establish_connection();
    /// #     diesel::insert_into(users).values(&NewUser::new("Jim"))
    /// #         .execute(&connection).unwrap();
    /// #[cfg(not(feature="postgres"))]
    /// let query = sql::<Text>("SELECT name FROM users WHERE id > ? AND name <> ?");
    /// #[cfg(feature="postgres")]
    /// let query = sql("SELECT name FROM users WHERE id > $1 AND name <> $2");
    /// let query = query
    ///     .bind::<Integer, _>(1)
    ///     .bind::<Text, _>("Jim");
    /// let expected = vec!["Tess".to_string()];
    /// assert_eq!(Ok(expected), query.load(&connection));
    /// # }
    /// ```
    #[deprecated(since = "0.99.0", note = "use `sql_query` if you need bind parameters")]
    #[cfg(feature = "with-deprecated")]
    pub fn bind<BindST, T>(self, bind_value: T) -> UncheckedBind<Self, T, BindST> {
        UncheckedBind::new(self, bind_value)
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
/// If you give the wrong type, it'll either crash at runtime when deserializing
/// the query result or produce invalid values.
///
/// # Examples
///
/// ```rust
/// # #[macro_use] extern crate diesel;
/// # #[macro_use] extern crate diesel_codegen;
/// use diesel::dsl::sql;
/// use diesel::types::{Bool, Integer, Text};
/// # include!("../doctest_setup.rs");
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
