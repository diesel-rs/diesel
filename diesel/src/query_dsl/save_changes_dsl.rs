use associations::Identifiable;
use backend::SupportsReturningClause;
use connection::Connection;
use helper_types::*;
use query_builder::{AsChangeset, IntoUpdateTarget};
use query_dsl::*;
use query_source::Queryable;
use result::QueryResult;
use types::HasSqlType;

pub trait SaveChangesDsl<Conn, ST> where
    Conn: Connection,
    Conn::Backend: HasSqlType<ST>,
{
    fn save_changes<T>(self, connection: &Conn) -> QueryResult<T> where
        T: Queryable<ST, Conn::Backend>;
}

impl<'a, T, ST, Conn> SaveChangesDsl<Conn, ST> for &'a T where
    Conn: Connection,
    Conn::Backend: HasSqlType<ST> + SupportsReturningClause,
    T: Identifiable,
    &'a T: AsChangeset<Target=T::Table> + IntoUpdateTarget<Table=T::Table>,
    Update<&'a T, &'a T>: LoadDsl<Conn, SqlType=ST>,
{
    fn save_changes<U>(self, conn: &Conn) -> QueryResult<U> where
        U: Queryable<ST, Conn::Backend>,
    {
        ::update(self).set(self).get_result(conn)
    }
}

#[cfg(feature = "sqlite")]
use sqlite::{SqliteConnection, Sqlite};
#[cfg(feature = "sqlite")]
use query_builder::AsQuery;

#[cfg(feature = "sqlite")]
impl<'a, T, ST> SaveChangesDsl<SqliteConnection, ST> for &'a T where
    Sqlite: HasSqlType<ST>,
    T: Identifiable,
    T::Table: AsQuery<SqlType=ST>,
    &'a T: AsChangeset<Target=T::Table> + IntoUpdateTarget<Table=T::Table>,
    Update<&'a T, &'a T>: ExecuteDsl<SqliteConnection>,
    Find<T::Table, T::Id>: LoadDsl<SqliteConnection, SqlType=ST>,
{
    fn save_changes<U>(self, conn: &SqliteConnection) -> QueryResult<U> where
        U: Queryable<ST, Sqlite>,
    {
        try!(::update(self).set(self).execute(conn));
        T::table().find(self.id()).get_result(conn)
    }
}
