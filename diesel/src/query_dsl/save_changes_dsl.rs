use associations::HasTable;
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

impl<T, ST, Conn> SaveChangesDsl<Conn, ST> for T where
    Conn: Connection,
    Conn::Backend: HasSqlType<ST> + SupportsReturningClause,
    T: Copy + AsChangeset<Target=<T as HasTable>::Table> + IntoUpdateTarget,
    Update<T, T>: LoadDsl<Conn, SqlType=ST>,
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
use associations::Identifiable;

#[cfg(feature = "sqlite")]
impl<T, ST> SaveChangesDsl<SqliteConnection, ST> for T where
    Sqlite: HasSqlType<ST>,
    T: Copy + Identifiable,
    T: AsChangeset<Target=<T as HasTable>::Table> + IntoUpdateTarget,
    T::Table: FindDsl<T::Id, SqlType=ST>,
    Update<T, T>: ExecuteDsl<SqliteConnection>,
    Find<T::Table, T::Id>: LoadDsl<SqliteConnection, SqlType=ST>,
{
    fn save_changes<U>(self, conn: &SqliteConnection) -> QueryResult<U> where
        U: Queryable<ST, Sqlite>,
    {
        try!(::update(self).set(self).execute(conn));
        T::table().find(self.id()).get_result(conn)
    }
}

#[cfg(feature = "mysql")]
use mysql::{MysqlConnection, Mysql};
#[cfg(feature = "mysql")]
use associations::Identifiable;

#[cfg(feature = "mysql")]
impl<T, ST> SaveChangesDsl<MysqlConnection, ST> for T where
    Mysql: HasSqlType<ST>,
    T: Copy + Identifiable,
    T: AsChangeset<Target=<T as HasTable>::Table> + IntoUpdateTarget,
    T::Table: FindDsl<T::Id, SqlType=ST>,
    Update<T, T>: ExecuteDsl<MysqlConnection>,
    Find<T::Table, T::Id>: LoadDsl<MysqlConnection, SqlType=ST>,
{
    fn save_changes<U>(self, conn: &MysqlConnection) -> QueryResult<U> where
        U: Queryable<ST, Mysql>,
    {
        try!(::update(self).set(self).execute(conn));
        T::table().find(self.id()).get_result(conn)
    }
}
