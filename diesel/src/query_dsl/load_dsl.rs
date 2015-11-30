use connection::{Connection, Cursor};
use query_builder::{Query, AsQuery};
use query_source::Queriable;
use result::QueryResult;
use super::LimitDsl;

/// Methods to execute a query given a connection. These are automatically implemented for the
/// various query types.
pub trait LoadDsl: AsQuery + LimitDsl + Sized {
    fn load<U>(self, conn: &Connection) -> QueryResult<Cursor<Self::SqlType, U>> where
        U: Queriable<Self::SqlType>
    {
        conn.query_all(self)
    }

    /// Attempts to load a single record. Returns `Ok(record)` if found, and
    /// `Err(NotFound)` if no results are returned. If the query truly is
    /// optional, you can call `.optional()` on the result of this to get a
    /// `Result<Option<U>>`.
    fn first<U>(self, conn: &Connection) -> QueryResult<U> where
        U: Queriable<<<Self as LimitDsl>::Output as Query>::SqlType>
    {
        conn.query_one(self.limit(1))
    }
}

impl<T: AsQuery + LimitDsl> LoadDsl for T {
}
