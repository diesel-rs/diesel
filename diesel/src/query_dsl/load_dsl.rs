use backend::Backend;
use connection::Connection;
use helper_types::Limit;
use query_builder::{QueryFragment, AsQuery, QueryId};
use query_source::Queryable;
use result::QueryResult;
use super::LimitDsl;
use types::HasSqlType;

/// Methods to execute a query given a connection. These are automatically implemented for the
/// various query types.
pub trait LoadDsl<Conn: Connection>: Sized where
    Conn::Backend: HasSqlType<Self::SqlType>,
{
    type SqlType;

    /// Executes the given query, returning a `Vec` with the returned rows.
    fn load<U>(self, conn: &Conn) -> QueryResult<Vec<U>> where
        U: Queryable<Self::SqlType, Conn::Backend>;

    /// Attempts to load a single record. Returns `Ok(record)` if found, and
    /// `Err(NotFound)` if no results are returned. If the query truly is
    /// optional, you can call `.optional()` on the result of this to get a
    /// `Result<Option<U>>`.
    fn first<U>(self, conn: &Conn) -> QueryResult<U> where
        Self: LimitDsl,
        Limit<Self>: LoadDsl<Conn, SqlType=<Self as LoadDsl<Conn>>::SqlType>,
        U: Queryable<<Self as LoadDsl<Conn>>::SqlType, Conn::Backend>,
    {
        self.limit(1).get_result(conn)
    }

    /// Runs the command, and returns the affected row. `Err(NotFound)` will be
    /// returned if the query affected 0 rows. You can call `.optional()` on the
    /// result of this if the command was optional to get back a
    /// `Result<Option<U>>`
    fn get_result<U>(self, conn: &Conn) -> QueryResult<U> where
        U: Queryable<Self::SqlType, Conn::Backend>;

    /// Runs the command, returning an `Vec` with the affected rows.
    fn get_results<U>(self, conn: &Conn) -> QueryResult<Vec<U>> where
        U: Queryable<Self::SqlType, Conn::Backend>,
    {
        self.load(conn)
    }
}

impl<Conn: Connection, T: AsQuery> LoadDsl<Conn> for T where
    T::Query: QueryFragment<Conn::Backend> + QueryId,
    Conn::Backend: HasSqlType<T::SqlType>,
{
    type SqlType = <Self as AsQuery>::SqlType;

    fn load<U>(self, conn: &Conn) -> QueryResult<Vec<U>> where
        U: Queryable<Self::SqlType, Conn::Backend>,
    {
        conn.query_all(self)
    }

    fn get_result<U>(self, conn: &Conn) -> QueryResult<U> where
        U: Queryable<Self::SqlType, Conn::Backend>,
    {
        conn.query_one(self)
    }
}

pub trait ExecuteDsl<Conn: Connection<Backend=DB>, DB: Backend = <Conn as Connection>::Backend>: Sized {
    /// Executes the given command, returning the number of rows affected. Used
    /// in conjunction with
    /// [`update`](/diesel/fn.update.html) and
    /// [`delete`](/diesel/fn.delete.html)
    fn execute(self, conn: &Conn) -> QueryResult<usize>;
}

impl<Conn, T> ExecuteDsl<Conn> for T where
    Conn: Connection,
    T: QueryFragment<Conn::Backend> + QueryId,
{
    fn execute(self, conn: &Conn) -> QueryResult<usize> {
        conn.execute_returning_count(&self)
    }
}
