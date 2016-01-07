use connection::{Connection, Cursor};
use query_builder::{Query, QueryFragment, AsQuery};
use query_source::Queriable;
use result::QueryResult;
use super::LimitDsl;
use db_result::DbResult;

/// Methods to execute a query given a connection. These are automatically implemented for the
/// various query types.
pub trait LoadDsl: AsQuery + Sized {
    /// Executes the given query, returning an `Iterator` over the returned
    /// rows.
    fn load<U, C>(self, conn: &C) -> QueryResult<Cursor<Self::SqlType, U, C::DbResult>> where
        U: Queriable<Self::SqlType>,
        C: Connection,
    {
        conn.query_all(self)
    }

    /// Attempts to load a single record. Returns `Ok(record)` if found, and
    /// `Err(NotFound)` if no results are returned. If the query truly is
    /// optional, you can call `.optional()` on the result of this to get a
    /// `Result<Option<U>>`.
    fn first<U, C>(self, conn: &C) -> QueryResult<U> where
        U: Queriable<<<Self as LimitDsl>::Output as Query>::SqlType>,
        Self: LimitDsl,
        C: Connection,
    {
        conn.query_one(self.limit(1))
    }

    /// Runs the command, and returns the affected row. `Err(NotFound)` will be
    /// returned if the query affected 0 rows. You can call `.optional()` on the
    /// result of this if the command was optional to get back a
    /// `Result<Option<U>>`
    fn get_result<U, C>(self, conn: &C) -> QueryResult<U> where
        U: Queriable<Self::SqlType>,
        C: Connection,
    {
        conn.query_one(self)
    }

    /// Runs the command, returning an `Iterator` over the affected rows.
    fn get_results<U, C, R>(self, conn: &C) -> QueryResult<Cursor<Self::SqlType, U, C::DbResult>> where
        U: Queriable<Self::SqlType>,
        C: Connection,
    {
        self.load(conn)
    }
}

impl<T: AsQuery> LoadDsl for T {
}

pub trait ExecuteDsl: QueryFragment + Sized {
    /// Executes the given command, returning the number of rows affected. Used
    /// in conjunction with
    /// [`update`](../query_builder/fn.update.html) and
    /// [`delete`](../query_builder/fn.delete.html)
    fn execute<C: Connection>(&self, conn: &C) -> QueryResult<usize> {
        conn.execute_returning_count(self)
    }
}

impl<T: QueryFragment> ExecuteDsl for T {
}
