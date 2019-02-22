use std::marker::PhantomData;

use backend::Backend;
use connection::Connection;
use deserialize::QueryableByName;
use query_builder::{AstPass, QueryFragment, QueryId};
use query_dsl::{LoadQuery, RunQueryDsl};
use result::QueryResult;
use serialize::ToSql;
use sql_types::HasSqlType;

#[derive(Debug, Clone)]
#[must_use = "Queries are only executed when calling `load`, `get_result` or similar."]
/// The return value of `sql_query`.
///
/// Unlike most queries in Diesel, `SqlQuery` loads its data by column name,
/// rather than by index. This means that you cannot deserialize this query into
/// a tuple, and any structs used must implement `QueryableByName`.
///
/// See [`sql_query`](../fn.sql_query.html) for examples.
pub struct SqlQuery<Inner = ()> {
    inner: Inner,
    query: String,
}

impl<Inner> SqlQuery<Inner> {
    pub(crate) fn new(inner: Inner, query: String) -> Self {
        SqlQuery { inner, query }
    }

    /// Bind a value for use with this SQL query.
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
    /// # #[macro_use] extern crate diesel;
    /// # include!("../doctest_setup.rs");
    /// #
    /// # use schema::users;
    /// #
    /// # #[derive(QueryableByName, Debug, PartialEq)]
    /// # #[table_name="users"]
    /// # struct User {
    /// #     id: i32,
    /// #     name: String,
    /// # }
    /// #
    /// # fn main() {
    /// #     use diesel::sql_query;
    /// #     use diesel::sql_types::{Integer, Text};
    /// #
    /// #     let connection = establish_connection();
    /// #     diesel::insert_into(users::table)
    /// #         .values(users::name.eq("Jim"))
    /// #         .execute(&connection).unwrap();
    /// # #[cfg(feature = "postgres")]
    /// # let users = sql_query("SELECT * FROM users WHERE id > $1 AND name != $2");
    /// # #[cfg(not(feature = "postgres"))]
    /// let users = sql_query("SELECT * FROM users WHERE id > ? AND name <> ?")
    /// # ;
    /// # let users = users
    ///     .bind::<Integer, _>(1)
    ///     .bind::<Text, _>("Tess")
    ///     .get_results(&connection);
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

impl<DB, Inner> QueryFragment<DB> for SqlQuery<Inner>
where
    DB: Backend,
    Inner: QueryFragment<DB>,
{
    fn walk_ast(&self, mut out: AstPass<DB>) -> QueryResult<()> {
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

impl<Inner, Conn, T> LoadQuery<Conn, T> for SqlQuery<Inner>
where
    Conn: Connection,
    T: QueryableByName<Conn::Backend>,
    Self: QueryFragment<Conn::Backend>,
{
    fn internal_load(self, conn: &Conn) -> QueryResult<Vec<T>> {
        conn.query_by_name(&self)
    }
}

impl<Inner, Conn> RunQueryDsl<Conn> for SqlQuery<Inner> {}

#[derive(Debug, Clone, Copy)]
#[must_use = "Queries are only executed when calling `load`, `get_result` or similar."]
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
    DB: Backend + HasSqlType<ST>,
    Query: QueryFragment<DB>,
    Value: ToSql<ST, DB>,
{
    fn walk_ast(&self, mut out: AstPass<DB>) -> QueryResult<()> {
        self.query.walk_ast(out.reborrow())?;
        out.push_bind_param_value_only(&self.value)?;
        Ok(())
    }
}

impl<Conn, Query, Value, ST, T> LoadQuery<Conn, T> for UncheckedBind<Query, Value, ST>
where
    Conn: Connection,
    T: QueryableByName<Conn::Backend>,
    Self: QueryFragment<Conn::Backend> + QueryId,
{
    fn internal_load(self, conn: &Conn) -> QueryResult<Vec<T>> {
        conn.query_by_name(&self)
    }
}

impl<Conn, Query, Value, ST> RunQueryDsl<Conn> for UncheckedBind<Query, Value, ST> {}

#[must_use = "Queries are only executed when calling `load`, `get_result`, or similar."]
/// See [`SqlQuery::into_boxed`].
///
/// [`SqlQuery::into_boxed`]: ../struct.SqlQuery.html#method.into_boxed
#[allow(missing_debug_implementations)]
pub struct BoxedSqlQuery<'f, DB: Backend, Query> {
    query: Query,
    sql: String,
    binds: Vec<Box<dyn Fn(AstPass<DB>) -> QueryResult<()> + 'f>>,
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
    /// [`SqlQuery::bind`]: ../struct.SqlQuery.html#method.bind
    pub fn bind<BindSt, Value>(mut self, b: Value) -> Self
    where
        DB: HasSqlType<BindSt>,
        Value: ToSql<BindSt, DB> + 'f,
    {
        self.binds
            .push(Box::new(move |mut out| out.push_bind_param_value_only(&b)));
        self
    }

    /// See [`SqlQuery::sql`].
    ///
    /// [`SqlQuery::sql`]: ../struct.SqlQuery.html#method.sql
    pub fn sql<T: AsRef<str>>(mut self, sql: T) -> Self {
        self.sql += sql.as_ref();
        self
    }
}

impl<DB, Query> QueryFragment<DB> for BoxedSqlQuery<'_, DB, Query>
where
    DB: Backend,
    Query: QueryFragment<DB>,
{
    fn walk_ast(&self, mut out: AstPass<DB>) -> QueryResult<()> {
        out.unsafe_to_cache_prepared();
        self.query.walk_ast(out.reborrow())?;
        out.push_sql(&self.sql);

        for b in &self.binds {
            b(out.reborrow())?;
        }
        Ok(())
    }
}

impl<DB: Backend, Query> QueryId for BoxedSqlQuery<'_, DB, Query> {
    type QueryId = ();

    const HAS_STATIC_QUERY_ID: bool = false;
}

impl<Conn, T, Query> LoadQuery<Conn, T> for BoxedSqlQuery<'_, Conn::Backend, Query>
where
    Conn: Connection,
    T: QueryableByName<Conn::Backend>,
    Self: QueryFragment<Conn::Backend> + QueryId,
{
    fn internal_load(self, conn: &Conn) -> QueryResult<Vec<T>> {
        conn.query_by_name(&self)
    }
}

impl<Conn: Connection, Query> RunQueryDsl<Conn> for BoxedSqlQuery<'_, Conn::Backend, Query> {}
