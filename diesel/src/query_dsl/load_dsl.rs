use connection::Connection;
use helper_types::Limit;
use query_builder::{QueryFragment, AsQuery, QueryId};
use query_source::Queryable;
use result::QueryResult;
use super::LimitDsl;
use types::HasSqlType;

/// Methods to execute a query given a connection. These are automatically implemented for the
/// various query types.
pub trait LoadDsl<Conn: Connection>: AsQuery + Sized where
    Conn::Backend: HasSqlType<Self::SqlType>,
{
    /// Executes the given query, returning an `Iterator` over the returned
    /// rows.
    fn load<'a, U>(self, conn: &Conn) -> QueryResult<Vec<U>> where
        U: Queryable<Self::SqlType, Conn::Backend> + 'a;

    /// Attempts to load a single record. Returns `Ok(record)` if found, and
    /// `Err(NotFound)` if no results are returned. If the query truly is
    /// optional, you can call `.optional()` on the result of this to get a
    /// `Result<Option<U>>`.
    fn first<U>(self, conn: &Conn) -> QueryResult<U> where
        Self: LimitDsl,
        Limit<Self>: LoadDsl<Conn, SqlType=Self::SqlType>,
        U: Queryable<Self::SqlType, Conn::Backend>,
    {
        self.limit(1).get_result(conn)
    }

    /// Runs the command, and returns the affected row. `Err(NotFound)` will be
    /// returned if the query affected 0 rows. You can call `.optional()` on the
    /// result of this if the command was optional to get back a
    /// `Result<Option<U>>`
    fn get_result<U>(self, conn: &Conn) -> QueryResult<U> where
        U: Queryable<Self::SqlType, Conn::Backend>;

    /// Runs the command, returning an `Iterator` over the affected rows.
    fn get_results<'a, U>(self, conn: &Conn) -> QueryResult<Vec<U>> where
        U: Queryable<Self::SqlType, Conn::Backend> + 'a,
    {
        self.load(conn)
    }
}

impl<Conn: Connection, T: AsQuery> LoadDsl<Conn> for T where
    T::Query: QueryFragment<Conn::Backend> + QueryId,
    Conn::Backend: HasSqlType<T::SqlType>,
{
    fn load<'a, U>(self, conn: &Conn) -> QueryResult<Vec<U>> where
        U: Queryable<Self::SqlType, Conn::Backend> + 'a,
    {
        conn.query_all(self)
    }

    fn get_result<U>(self, conn: &Conn) -> QueryResult<U> where
        U: Queryable<Self::SqlType, Conn::Backend>,
    {
        conn.query_one(self)
    }
}

pub trait ExecuteDsl<Conn: Connection>: Sized + QueryFragment<Conn::Backend> + QueryId {
    /// Executes the given command, returning the number of rows affected. Used
    /// in conjunction with
    /// [`update`](../query_builder/fn.update.html) and
    /// [`delete`](../query_builder/fn.delete.html)
    fn execute(&self, conn: &Conn) -> QueryResult<usize> {
        // Skip query if it doesn't contain any values
        if self.is_empty() {
            return Ok(0);
        }

        conn.execute_returning_count(self)
    }
}

impl<Conn, T> ExecuteDsl<Conn> for T where
    Conn: Connection,
    T: QueryFragment<Conn::Backend> + QueryId,
{
}
