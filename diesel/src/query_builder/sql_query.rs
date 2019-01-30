use std::fmt::{self, Debug};
use std::marker::PhantomData;

use backend::Backend;
use connection::Connection;
use deserialize::QueryableByName;
use query_builder::{AstPass, QueryFragment, QueryId, self, debug_query};
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
pub struct SqlQuery {
    query: String,
}

impl SqlQuery {
    pub(crate) fn new(query: String) -> Self {
        SqlQuery { query }
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
}

impl<DB> QueryFragment<DB> for SqlQuery
where
    DB: Backend,
{
    fn walk_ast(&self, mut out: AstPass<DB>) -> QueryResult<()> {
        out.unsafe_to_cache_prepared();
        out.push_sql(&self.query);
        Ok(())
    }
}

impl QueryId for SqlQuery {
    type QueryId = ();

    const HAS_STATIC_QUERY_ID: bool = false;
}

impl<Conn, T> LoadQuery<Conn, T> for SqlQuery
where
    Conn: Connection,
    T: QueryableByName<Conn::Backend>,
{
    fn internal_load(self, conn: &Conn) -> QueryResult<Vec<T>> {
        conn.query_by_name(&self)
    }
}

impl<Conn> RunQueryDsl<Conn> for SqlQuery {}

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
/// See `SqlQuery`
pub struct BoxedSqlQuery<'f, DB: Backend, Inner> {
    inner: Inner,
    sql: String,
    binds: Vec<Box<dyn Fn(AstPass<DB>) -> QueryResult<()> + 'f>>,
}

impl<DB: Backend, Inner> Debug for BoxedSqlQuery<'_, DB, Inner>
where
    for<'a> debug_query::DebugQuery<'a, Self, DB>: Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        query_builder::debug_query::<DB, _>(self).fmt(f)
    }
}

impl<'f, DB: Backend, Inner> BoxedSqlQuery<'f, DB, Inner> {
    pub(crate) fn new(inner: Inner) -> Self {
        BoxedSqlQuery {
            inner,
            sql: "".to_string(),
            binds: vec![],
        }
    }

    /// See `SqlQuery`'s `bind`
    pub fn bind<BindSt, Value>(mut self, b: Value) -> Self
    where
        BindSt: QueryId,
        DB: HasSqlType<BindSt>,
        Value: ToSql<BindSt, DB> + 'f,
    {
        self.binds.push(Box::new(move |mut out| {
            out.push_bind_param_value_only(&b)
        }));
        self
    }

    /// See `SqlQuery`'s `sql`
    pub fn sql(mut self, sql: &str) -> Self {
        self.sql += sql;
        self
    }
}

impl<DB, Inner> QueryFragment<DB> for BoxedSqlQuery<'_, DB, Inner>
where
    DB: Backend,
    Inner: QueryFragment<DB>,
{
    fn walk_ast(&self, mut out: AstPass<DB>) -> QueryResult<()> {
        out.unsafe_to_cache_prepared();
        self.inner.walk_ast(out.reborrow())?;
        out.push_sql(&self.sql);

        for b in &self.binds {
            b(out.reborrow())?;
        }
        Ok(())
    }
}

impl<DB: Backend, Inner> QueryId for BoxedSqlQuery<'_, DB, Inner> {
    type QueryId = ();

    const HAS_STATIC_QUERY_ID: bool = false;
}

impl<Conn, T, Inner> LoadQuery<Conn, T> for BoxedSqlQuery<'_, Conn::Backend, Inner>
where
    Conn: Connection,
    T: QueryableByName<Conn::Backend>,
    Self: QueryFragment<Conn::Backend> + QueryId,
{
    fn internal_load(self, conn: &Conn) -> QueryResult<Vec<T>> {
        conn.query_by_name(&self)
    }
}

impl<Conn: Connection, Inner> RunQueryDsl<Conn> for BoxedSqlQuery<'_, Conn::Backend, Inner> {}
