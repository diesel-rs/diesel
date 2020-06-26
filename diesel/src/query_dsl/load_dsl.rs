use super::RunQueryDsl;
use crate::backend::Backend;
use crate::connection::Connection;
use crate::deserialize::FromSqlRow;
use crate::expression::QueryMetadata;
use crate::query_builder::{AsQuery, QueryFragment, QueryId};
use crate::result::QueryResult;

/// The `load` method
///
/// This trait should not be relied on directly by most apps. Its behavior is
/// provided by [`RunQueryDsl`]. However, you may need a where clause on this trait
/// to call `load` from generic code.
///
/// [`RunQueryDsl`]: ../trait.RunQueryDsl.html
pub trait LoadQuery<Conn, U>: RunQueryDsl<Conn> {
    /// Load this query
    fn internal_load(self, conn: &Conn) -> QueryResult<Vec<U>>;
}

impl<Conn, T, U> LoadQuery<Conn, U> for T
where
    Conn: Connection,
    T: AsQuery + RunQueryDsl<Conn>,
    T::Query: QueryFragment<Conn::Backend> + QueryId,
    U: FromSqlRow<T::SqlType, Conn::Backend>,
    Conn::Backend: QueryMetadata<T::SqlType>,
{
    fn internal_load(self, conn: &Conn) -> QueryResult<Vec<U>> {
        conn.load(self)
    }
}

/// The `execute` method
///
/// This trait should not be relied on directly by most apps. Its behavior is
/// provided by [`RunQueryDsl`]. However, you may need a where clause on this trait
/// to call `execute` from generic code.
///
/// [`RunQueryDsl`]: ../trait.RunQueryDsl.html
pub trait ExecuteDsl<Conn: Connection<Backend = DB>, DB: Backend = <Conn as Connection>::Backend>:
    Sized
{
    /// Execute this command
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
