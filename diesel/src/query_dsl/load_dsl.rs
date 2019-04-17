use super::RunQueryDsl;
use backend::Backend;
use connection::Connection;
use deserialize::Queryable;
use query_builder::{AsQuery, NamedQueryFragment, QueryFragment, QueryId};
use result::QueryResult;
use sql_types::HasSqlType;

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
    Conn::Backend: HasSqlType<T::SqlType>,
    T: AsQuery + RunQueryDsl<Conn>,
    T::Query: QueryFragment<Conn::Backend> + QueryId,
    U: Queryable<T::SqlType, Conn::Backend>,
{
    fn internal_load(self, conn: &Conn) -> QueryResult<Vec<U>> {
        conn.query_by_index(self)
    }
}

use deserialize::{LabeledQueryableWrapper, MapToQueryType, NamedQueryable, IntoHlist};

pub fn labelled_query<T, U, Conn, P1, P2>(query: T, conn: &Conn) -> QueryResult<Vec<U>>
where
    Conn: Connection,
    Conn::Backend: HasSqlType<T::SqlType>,
    T: AsQuery + RunQueryDsl<Conn>,
    T::Query: QueryFragment<Conn::Backend> + QueryId + NamedQueryFragment,
    <T::Query as NamedQueryFragment>::Name: IntoHlist,
    U: NamedQueryable,
    U::Row: MapToQueryType<<T::Query as NamedQueryFragment>::Name, P1>,
    LabeledQueryableWrapper<
        <U::Row as MapToQueryType<<T::Query as NamedQueryFragment>::Name, P1>>::Queryable,
        U,
        <<T::Query as NamedQueryFragment>::Name as IntoHlist>::Hlist,
        P2,
    >: Queryable<T::SqlType, Conn::Backend>,
{
    let raw: Vec<LabeledQueryableWrapper<_, U, _, _>> = query.internal_load(conn)?;
    // This is actually safe because LabeledQueryableWrapper has 2 fields, one of the type U
    // and one of the type PhantomData which is actually a zero sized type
    // Additionally LabeledQueryableWrapper is #[repr(C)], therefore it is safe to assume
    // that the type is binary compatible with the inner U field, which means we could
    // just tell the compiler it is U
    Ok(unsafe { std::mem::transmute(raw) })
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
