use connection::Connection;
use query_builder::{Query, QueryFragment, AsQuery};
use query_source::Queryable;
use result::QueryResult;
use super::LimitDsl;

/// Methods to execute a query given a connection. These are automatically implemented for the
/// various query types.
pub trait LoadDsl<Conn: Connection>: AsQuery + Sized where
    Self::Query: QueryFragment<Conn::Backend>,
{
    /// Executes the given query, returning an `Iterator` over the returned
    /// rows.
    fn load<'a, U>(self, conn: &Conn) -> QueryResult<Box<Iterator<Item=U> + 'a>> where
        U: Queryable<Self::SqlType> + 'a,
    {
        conn.query_all(self)
    }

    /// Attempts to load a single record. Returns `Ok(record)` if found, and
    /// `Err(NotFound)` if no results are returned. If the query truly is
    /// optional, you can call `.optional()` on the result of this to get a
    /// `Result<Option<U>>`.
    fn first<U>(self, conn: &Conn) -> QueryResult<U> where
        U: Queryable<<<Self as LimitDsl>::Output as Query>::SqlType>,
        Self: LimitDsl,
        <Self as LimitDsl>::Output: QueryFragment<Conn::Backend>,
    {
        conn.query_one(self.limit(1))
    }

    /// Runs the command, and returns the affected row. `Err(NotFound)` will be
    /// returned if the query affected 0 rows. You can call `.optional()` on the
    /// result of this if the command was optional to get back a
    /// `Result<Option<U>>`
    fn get_result<U>(self, conn: &Conn) -> QueryResult<U> where
        U: Queryable<Self::SqlType>,
    {
        conn.query_one(self)
    }

    /// Runs the command, returning an `Iterator` over the affected rows.
    fn get_results<'a, U>(self, conn: &Conn) -> QueryResult<Box<Iterator<Item=U> + 'a>> where
        U: Queryable<Self::SqlType> + 'a,
    {
        self.load(conn)
    }
}

impl<Conn: Connection, T: AsQuery> LoadDsl<Conn> for T where
    T::Query: QueryFragment<Conn::Backend>,
{
}

pub trait ExecuteDsl<Conn: Connection>: Sized + QueryFragment<Conn::Backend> {
    /// Executes the given command, returning the number of rows affected. Used
    /// in conjunction with
    /// [`update`](../query_builder/fn.update.html) and
    /// [`delete`](../query_builder/fn.delete.html)
    fn execute(&self, conn: &Conn) -> QueryResult<usize> {
        conn.execute_returning_count(self)
    }
}

impl<Conn, T> ExecuteDsl<Conn> for T where
    Conn: Connection,
    T: QueryFragment<Conn::Backend>
{
}
