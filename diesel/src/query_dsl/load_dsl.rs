use super::RunQueryDsl;
use crate::backend::Backend;
use crate::connection::Connection;
use crate::deserialize::FromSqlRow;
use crate::expression::{select_by::SelectBy, Expression, QueryMetadata, Selectable};
use crate::query_builder::{AsQuery, QueryFragment, QueryId};
use crate::result::QueryResult;

/// The `load` method
///
/// This trait should not be relied on directly by most apps. Its behavior is
/// provided by [`RunQueryDsl`]. However, you may need a where clause on this trait
/// to call `load` from generic code.
///
/// [`RunQueryDsl`]: crate::RunQueryDsl
pub trait LoadQuery<Conn, U>: RunQueryDsl<Conn> {
    /// Load this query
    fn internal_load(self, conn: &mut Conn) -> QueryResult<Vec<U>>;
}

use crate::expression::TypedExpressionType;
use crate::sql_types::{SqlType, Untyped};

pub trait CompatibleType<U, DB> {
    type SqlType;
}

impl<ST, U, DB> CompatibleType<U, DB> for ST
where
    DB: Backend,
    ST: SqlType + crate::sql_types::SingleValue,
    U: FromSqlRow<ST, DB>,
{
    type SqlType = ST;
}

impl<U, DB> CompatibleType<U, DB> for Untyped
where
    U: FromSqlRow<Untyped, DB>,
    DB: Backend,
{
    type SqlType = Untyped;
}

impl<U, DB, E, ST> CompatibleType<U, DB> for SelectBy<U, DB>
where
    DB: Backend,
    ST: SqlType + TypedExpressionType,
    U: Selectable<DB, SelectExpression = E>,
    E: Expression<SqlType = ST>,
    U: FromSqlRow<ST, DB>,
{
    type SqlType = ST;
}

impl<Conn, T, U> LoadQuery<Conn, U> for T
where
    Conn: Connection,
    T: AsQuery + RunQueryDsl<Conn>,
    T::Query: QueryFragment<Conn::Backend> + QueryId,
    T::SqlType: CompatibleType<U, Conn::Backend>,
    Conn::Backend: QueryMetadata<T::SqlType>,
    U: FromSqlRow<<T::SqlType as CompatibleType<U, Conn::Backend>>::SqlType, Conn::Backend>,
{
    fn internal_load(self, conn: &mut Conn) -> QueryResult<Vec<U>> {
        conn.load(self)
    }
}

/// The `execute` method
///
/// This trait should not be relied on directly by most apps. Its behavior is
/// provided by [`RunQueryDsl`]. However, you may need a where clause on this trait
/// to call `execute` from generic code.
///
/// [`RunQueryDsl`]: crate::RunQueryDsl
pub trait ExecuteDsl<Conn: Connection<Backend = DB>, DB: Backend = <Conn as Connection>::Backend>:
    Sized
{
    /// Execute this command
    fn execute(query: Self, conn: &mut Conn) -> QueryResult<usize>;
}

impl<Conn, DB, T> ExecuteDsl<Conn, DB> for T
where
    Conn: Connection<Backend = DB>,
    DB: Backend,
    T: QueryFragment<DB> + QueryId,
{
    fn execute(query: Self, conn: &mut Conn) -> QueryResult<usize> {
        conn.execute_returning_count(&query)
    }
}
