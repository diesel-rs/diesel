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
