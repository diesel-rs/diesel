use backend::Backend;
use connection::Connection;
use query_builder::{AsQuery, QueryFragment, QueryId};
use query_source::Queryable;
use result::QueryResult;
use super::RunQueryDsl;
use types::HasSqlType;

pub trait LoadQuery<Conn, U>: RunQueryDsl<Conn> {
    fn internal_load(self, conn: &Conn) -> QueryResult<Vec<U>>;
}

impl<Conn, T, U> LoadQuery<Conn, U> for T
where
    Conn: Connection,
    Conn::Backend: HasSqlType<T::SqlType>,
    T: AsQuery + RunQueryDsl<Conn>,
    T::Query: QueryFragment<Conn::Backend> + QueryId,
    U: Queryable<T::SqlType, Conn::Backend>,
{
    fn internal_load(self, conn: &Conn) -> QueryResult<Vec<U>> {
        conn.query_by_index(self)
    }
}

/// Methods to execute a query given a connection. These are automatically implemented for the
/// various query types.
pub trait LoadDsl<Conn>: Sized {
    /// Executes the given query, returning a `Vec` with the returned rows.
    fn load<U>(self, conn: &Conn) -> QueryResult<Vec<U>>
    where
        Self: LoadQuery<Conn, U>,
    {
        self.internal_load(conn)
    }

    /// Runs the command, and returns the affected row. `Err(NotFound)` will be
    /// returned if the query affected 0 rows. You can call `.optional()` on the
    /// result of this if the command was optional to get back a
    /// `Result<Option<U>>`
    fn get_result<U>(self, conn: &Conn) -> QueryResult<U>
    where
        Self: LoadQuery<Conn, U>,
    {
        first_or_not_found(self.load(conn))
    }

    /// Runs the command, returning an `Vec` with the affected rows.
    fn get_results<U>(self, conn: &Conn) -> QueryResult<Vec<U>>
    where
        Self: LoadQuery<Conn, U>,
    {
        self.load(conn)
    }
}

impl<Conn, T> LoadDsl<Conn> for T
where
    Conn: Connection,
    Conn::Backend: HasSqlType<T::SqlType>,
    T: AsQuery,
    T::Query: QueryFragment<Conn::Backend> + QueryId,
{
}

pub trait FirstDsl<Conn>: LimitDsl + LoadDsl<Conn> {
    /// Attempts to load a single record. Returns `Ok(record)` if found, and
    /// `Err(NotFound)` if no results are returned. If the query truly is
    /// optional, you can call `.optional()` on the result of this to get a
    /// `Result<Option<U>>`.
    ///
    /// # Example:
    ///
    /// ```rust
    /// # #[macro_use] extern crate diesel;
    /// # include!("../doctest_setup.rs");
    /// # use diesel::NotFound;
    /// table! {
    ///     users {
    ///         id -> Integer,
    ///         name -> VarChar,
    ///     }
    /// }
    ///
    /// #[derive(Queryable, PartialEq, Debug)]
    /// struct User {
    ///     id: i32,
    ///     name: String,
    /// }
    ///
    /// # fn main() {
    /// #   let connection = establish_connection();
    /// let user1 = NewUser { name: "Sean".into() };
    /// let user2 = NewUser { name: "Pascal".into() };
    /// diesel::insert_into(users::table).values(&vec![user1, user2]).execute(&connection).unwrap();
    ///
    /// let user = users::table.order(users::id.asc()).first(&connection);
    /// assert_eq!(Ok(User { id: 1, name: "Sean".into() }), user);
    /// let user = users::table.filter(users::name.eq("Foo")).first::<User>(&connection);
    /// assert_eq!(Err(NotFound), user);
    /// # }
    /// ```
    fn first<U>(self, conn: &Conn) -> QueryResult<U>
    where
        Limit<Self>: LoadQuery<Conn, U>,
    {
        self.limit(1).get_result(conn)
    }
}

impl<Conn, T: LimitDsl + LoadDsl<Conn>> FirstDsl<Conn> for T {}

pub trait ExecuteDsl<Conn: Connection<Backend = DB>, DB: Backend = <Conn as Connection>::Backend>
    : Sized {
    fn execute(query: Self, conn: &Conn) -> QueryResult<usize>;
}

impl<Conn, DB, T> ExecuteDsl<Conn, DB> for T
where
    Conn: Connection<Backend = DB>,
    DB: Backend,
    T: QueryFragment<DB> + QueryId,
{
    fn execute(query: Self, conn: &Conn) -> QueryResult<usize> {
        conn.execute_returning_count(&query)
    }
}
