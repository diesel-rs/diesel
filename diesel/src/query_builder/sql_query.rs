use std::marker::PhantomData;

use super::Query;
use crate::backend::{Backend, DieselReserveSpecialization};
use crate::connection::Connection;
use crate::query_builder::{AstPass, QueryFragment, QueryId};
use crate::query_dsl::RunQueryDsl;
use crate::result::QueryResult;
use crate::serialize::ToSql;
use crate::sql_types::{HasSqlType, Untyped};

#[derive(Debug, Clone)]
#[must_use = "Queries are only executed when calling `load`, `get_result` or similar."]
/// The return value of `sql_query`.
///
/// Unlike most queries in Diesel, `SqlQuery` loads its data by column name,
/// rather than by index. This means that you cannot deserialize this query into
/// a tuple, and any structs used must implement `QueryableByName`.
///
/// See [`sql_query`](crate::sql_query()) for examples.
pub struct SqlQuery<Inner = self::private::Empty> {
    inner: Inner,
    query: String,
}

impl<Inner> SqlQuery<Inner> {
    pub(crate) fn new(inner: Inner, query: String) -> Self {
        SqlQuery { inner, query }
    }

    /// Bind a value for use with this SQL query. The given query should have
    /// placeholders that vary based on the database type,
    /// like [SQLite Parameter](https://sqlite.org/lang_expr.html#varparam) syntax,
    /// [PostgreSQL PREPARE syntax](https://www.postgresql.org/docs/current/sql-prepare.html),
    /// or [MySQL bind syntax](https://dev.mysql.com/doc/refman/8.0/en/mysql-stmt-bind-param.html).
    ///
    /// # Safety
    ///
    /// This function should be used with care, as Diesel cannot validate that
    /// the value is of the right type nor can it validate that you have passed
    /// the correct number of parameters.
    ///
    /// # Example
    ///
    /// ```
    /// # include!("../doctest_setup.rs");
    /// #
    /// # use schema::users;
    /// #
    /// # #[derive(QueryableByName, Debug, PartialEq)]
    /// # struct User {
    /// #     id: i32,
    /// #     name: String,
    /// # }
    /// #
    /// # fn main() {
    /// #     use diesel::sql_query;
    /// #     use diesel::sql_types::{Integer, Text};
    /// #
    /// #     let connection = &mut establish_connection();
    /// #     diesel::insert_into(users::table)
    /// #         .values(users::name.eq("Jim"))
    /// #         .execute(connection).unwrap();
    /// # #[cfg(feature = "postgres")]
    /// # let users = sql_query("SELECT * FROM users WHERE id > $1 AND name != $2");
    /// # #[cfg(not(feature = "postgres"))]
    /// let users = sql_query("SELECT * FROM users WHERE id > ? AND name <> ?")
    /// # ;
    /// # let users = users
    ///     .bind::<Integer, _>(1)
    ///     .bind::<Text, _>("Tess")
    ///     .get_results(connection);
    /// let expected_users = vec![
    ///     User { id: 3, name: "Jim".into() },
    /// ];
    /// assert_eq!(Ok(expected_users), users);
    /// # }
    /// ```
    pub fn bind<ST, Value>(self, value: Value) -> UncheckedBind<Self, Value, ST> {
        UncheckedBind::new(self, value)
    }

    /// Internally boxes future calls on `bind` and `sql` so that they don't
    /// change the type.
    ///
    /// This allows doing things you otherwise couldn't do, e.g. `bind`ing in a
    /// loop.
    pub fn into_boxed<'f, DB: Backend>(self) -> BoxedSqlQuery<'f, DB, Self> {
        BoxedSqlQuery::new(self)
    }

    /// Appends a piece of SQL code at the end.
    pub fn sql<T: AsRef<str>>(mut self, sql: T) -> Self {
        self.query += sql.as_ref();
        self
    }
}

impl SqlQuery {
    pub(crate) fn from_sql(query: String) -> SqlQuery {
        Self {
            inner: self::private::Empty,
            query,
        }
    }
}

impl<DB, Inner> QueryFragment<DB> for SqlQuery<Inner>
where
    DB: Backend + DieselReserveSpecialization,
    Inner: QueryFragment<DB>,
{
    fn walk_ast<'b>(&'b self, mut out: AstPass<'_, 'b, DB>) -> QueryResult<()> {
        out.unsafe_to_cache_prepared();
        self.inner.walk_ast(out.reborrow())?;
        out.push_sql(&self.query);
        Ok(())
    }
}

impl<Inner> QueryId for SqlQuery<Inner> {
    type QueryId = ();

    const HAS_STATIC_QUERY_ID: bool = false;
}

impl<Inner> Query for SqlQuery<Inner> {
    type SqlType = Untyped;
}

impl<Inner, Conn> RunQueryDsl<Conn> for SqlQuery<Inner> {}

#[derive(Debug, Clone, Copy)]
#[must_use = "Queries are only executed when calling `load`, `get_result` or similar."]
/// Returned by the [`SqlQuery::bind()`] method when binding a value to a fragment of SQL.
///
pub struct UncheckedBind<Query, Value, ST> {
    query: Query,
    value: Value,
    _marker: PhantomData<ST>,
}

impl<Query, Value, ST> UncheckedBind<Query, Value, ST> {
    pub fn new(query: Query, value: Value) -> Self {
        UncheckedBind {
            query,
            value,
            _marker: PhantomData,
        }
    }

    pub fn bind<ST2, Value2>(self, value: Value2) -> UncheckedBind<Self, Value2, ST2> {
        UncheckedBind::new(self, value)
    }

    pub fn into_boxed<'f, DB: Backend>(self) -> BoxedSqlQuery<'f, DB, Self> {
        BoxedSqlQuery::new(self)
    }

    /// Construct a full SQL query using raw SQL.
    ///
    /// This function exists for cases where a query needs to be written that is not
    /// supported by the query builder. Unlike most queries in Diesel, `sql_query`
    /// will deserialize its data by name, not by index. That means that you cannot
    /// deserialize into a tuple, and structs which you deserialize from this
    /// function will need to have `#[derive(QueryableByName)]`.
    ///
    /// This function is intended for use when you want to write the entire query
    /// using raw SQL. If you only need a small bit of raw SQL in your query, use
    /// [`sql`](dsl::sql()) instead.
    ///
    /// Query parameters can be bound into the raw query using [`SqlQuery::bind()`].
    ///
    /// # Safety
    ///
    /// The implementation of `QueryableByName` will assume that columns with a
    /// given name will have a certain type. The compiler will be unable to verify
    /// that the given type is correct. If your query returns a column of an
    /// unexpected type, the result may have the wrong value, or return an error.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # include!("../doctest_setup.rs");
    /// #
    /// # use schema::users;
    /// #
    /// # #[derive(QueryableByName, Debug, PartialEq)]
    /// # struct User {
    /// #     id: i32,
    /// #     name: String,
    /// # }
    /// #
    /// # fn main() {
    /// #     use diesel::sql_query;
    /// #     use diesel::sql_types::{Integer, Text};
    /// #
    /// #     let connection = &mut establish_connection();
    /// #     diesel::insert_into(users::table)
    /// #         .values(users::name.eq("Jim"))
    /// #         .execute(connection).unwrap();
    /// # #[cfg(feature = "postgres")]
    /// # let users = sql_query("SELECT * FROM users WHERE id > $1 AND name != $2");
    /// # #[cfg(not(feature = "postgres"))]
    /// let users = sql_query("SELECT * FROM users WHERE id > ? AND name <> ?")
    /// # ;
    /// # let users = users
    ///     .bind::<Integer, _>(1)
    ///     .bind::<Text, _>("Tess")
    ///     .get_results(connection);
    /// let expected_users = vec![
    ///     User { id: 3, name: "Jim".into() },
    /// ];
    /// assert_eq!(Ok(expected_users), users);
    /// # }
    /// ```
    /// [`SqlQuery::bind()`]: query_builder::SqlQuery::bind()
    pub fn sql<T: Into<String>>(self, sql: T) -> SqlQuery<Self> {
        SqlQuery::new(self, sql.into())
    }
}

impl<Query, Value, ST> QueryId for UncheckedBind<Query, Value, ST>
where
    Query: QueryId,
    ST: QueryId,
{
    type QueryId = UncheckedBind<Query::QueryId, (), ST::QueryId>;

    const HAS_STATIC_QUERY_ID: bool = Query::HAS_STATIC_QUERY_ID && ST::HAS_STATIC_QUERY_ID;
}

impl<Query, Value, ST, DB> QueryFragment<DB> for UncheckedBind<Query, Value, ST>
where
    DB: Backend + HasSqlType<ST> + DieselReserveSpecialization,
    Query: QueryFragment<DB>,
    Value: ToSql<ST, DB>,
{
    fn walk_ast<'b>(&'b self, mut out: AstPass<'_, 'b, DB>) -> QueryResult<()> {
        self.query.walk_ast(out.reborrow())?;
        out.push_bind_param_value_only(&self.value)?;
        Ok(())
    }
}

impl<Q, Value, ST> Query for UncheckedBind<Q, Value, ST> {
    type SqlType = Untyped;
}

impl<Conn, Query, Value, ST> RunQueryDsl<Conn> for UncheckedBind<Query, Value, ST> {}

#[must_use = "Queries are only executed when calling `load`, `get_result`, or similar."]
/// See [`SqlQuery::into_boxed`].
///
/// [`SqlQuery::into_boxed`]: SqlQuery::into_boxed()
#[allow(missing_debug_implementations)]
pub struct BoxedSqlQuery<'f, DB: Backend, Query> {
    query: Query,
    sql: String,
    binds: Vec<Box<dyn QueryFragment<DB> + Send + 'f>>,
}

struct RawBind<ST, U> {
    value: U,
    p: PhantomData<ST>,
}

impl<ST, U, DB> QueryFragment<DB> for RawBind<ST, U>
where
    DB: Backend + HasSqlType<ST>,
    U: ToSql<ST, DB>,
{
    fn walk_ast<'b>(&'b self, mut pass: AstPass<'_, 'b, DB>) -> QueryResult<()> {
        pass.push_bind_param_value_only(&self.value)
    }
}

impl<'f, DB: Backend, Query> BoxedSqlQuery<'f, DB, Query> {
    pub(crate) fn new(query: Query) -> Self {
        BoxedSqlQuery {
            query,
            sql: "".to_string(),
            binds: vec![],
        }
    }

    /// See [`SqlQuery::bind`].
    ///
    /// [`SqlQuery::bind`]: SqlQuery::bind()
    pub fn bind<BindSt, Value>(mut self, b: Value) -> Self
    where
        DB: HasSqlType<BindSt>,
        Value: ToSql<BindSt, DB> + Send + 'f,
        BindSt: Send + 'f,
    {
        self.binds.push(Box::new(RawBind {
            value: b,
            p: PhantomData,
        }) as Box<_>);
        self
    }

    /// See [`SqlQuery::sql`].
    ///
    /// [`SqlQuery::sql`]: SqlQuery::sql()
    pub fn sql<T: AsRef<str>>(mut self, sql: T) -> Self {
        self.sql += sql.as_ref();
        self
    }
}

impl<DB, Query> QueryFragment<DB> for BoxedSqlQuery<'_, DB, Query>
where
    DB: Backend + DieselReserveSpecialization,
    Query: QueryFragment<DB>,
{
    fn walk_ast<'b>(&'b self, mut out: AstPass<'_, 'b, DB>) -> QueryResult<()> {
        out.unsafe_to_cache_prepared();
        self.query.walk_ast(out.reborrow())?;
        out.push_sql(&self.sql);

        for b in &self.binds {
            b.walk_ast(out.reborrow())?;
        }
        Ok(())
    }
}

impl<DB: Backend, Query> QueryId for BoxedSqlQuery<'_, DB, Query> {
    type QueryId = ();

    const HAS_STATIC_QUERY_ID: bool = false;
}

impl<DB, Q> Query for BoxedSqlQuery<'_, DB, Q>
where
    DB: Backend,
{
    type SqlType = Untyped;
}

impl<Conn: Connection, Query> RunQueryDsl<Conn> for BoxedSqlQuery<'_, Conn::Backend, Query> {}

mod private {
    use crate::backend::{Backend, DieselReserveSpecialization};
    use crate::query_builder::{QueryFragment, QueryId};

    #[derive(Debug, Clone, Copy, QueryId)]
    pub struct Empty;

    impl<DB> QueryFragment<DB> for Empty
    where
        DB: Backend + DieselReserveSpecialization,
    {
        fn walk_ast<'b>(
            &'b self,
            _pass: crate::query_builder::AstPass<'_, 'b, DB>,
        ) -> crate::QueryResult<()> {
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    fn assert_send<S: Send>(_: S) {}

    #[test]
    fn check_boxed_sql_query_is_send() {
        let query = crate::sql_query("SELECT 1")
            .into_boxed::<<crate::test_helpers::TestConnection as crate::Connection>::Backend>(
        );

        assert_send(query);
    }
}
