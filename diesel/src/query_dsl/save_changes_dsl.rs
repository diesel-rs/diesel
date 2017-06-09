use associations::HasTable;
use helper_types::*;
use query_builder::{AsChangeset, IntoUpdateTarget};
use query_dsl::*;
use result::QueryResult;
#[cfg(any(feature = "sqlite", feature = "mysql"))]
use associations::Identifiable;

pub trait InternalSaveChangesDsl<Conn, T>: Sized {
    fn internal_save_changes(self, connection: &Conn) -> QueryResult<T>;
}

impl<T, U, Conn> InternalSaveChangesDsl<Conn, U> for T where
    T: Copy + AsChangeset<Target=<T as HasTable>::Table> + IntoUpdateTarget,
    Update<T, T>: LoadDsl<Conn> + LoadQuery<Conn, U>,
{
    fn internal_save_changes(self, conn: &Conn) -> QueryResult<U> {
        ::update(self).set(self).get_result(conn)
    }
}

#[cfg(feature = "sqlite")]
use sqlite::SqliteConnection;

#[cfg(feature = "sqlite")]
impl<T, U> InternalSaveChangesDsl<SqliteConnection, U> for T where
    T: Copy + Identifiable,
    T: AsChangeset<Target=<T as HasTable>::Table> + IntoUpdateTarget,
    T::Table: FindDsl<T::Id>,
    Update<T, T>: ExecuteDsl<SqliteConnection>,
    Find<T::Table, T::Id>: LoadQuery<SqliteConnection, U>,
{
    fn internal_save_changes(self, conn: &SqliteConnection) -> QueryResult<U> {
        try!(::update(self).set(self).execute(conn));
        T::table().find(self.id()).get_result(conn)
    }
}

#[cfg(feature = "mysql")]
use mysql::MysqlConnection;

#[cfg(feature = "mysql")]
impl<T, U> InternalSaveChangesDsl<MysqlConnection, U> for T where
    T: Copy + Identifiable,
    T: AsChangeset<Target=<T as HasTable>::Table> + IntoUpdateTarget,
    T::Table: FindDsl<T::Id>,
    Update<T, T>: ExecuteDsl<MysqlConnection>,
    Find<T::Table, T::Id>: LoadQuery<MysqlConnection, U>,
{
    fn internal_save_changes(self, conn: &MysqlConnection) -> QueryResult<U> {
        try!(::update(self).set(self).execute(conn));
        T::table().find(self.id()).get_result(conn)
    }
}

pub trait SaveChangesDsl<Conn> {
    fn save_changes<T>(self, connection: &Conn) -> QueryResult<T> where
        Self: InternalSaveChangesDsl<Conn, T>,
    {
        self.internal_save_changes(connection)
    }
}

impl<T, Conn> SaveChangesDsl<Conn> for T where
    T: Copy + AsChangeset<Target=<T as HasTable>::Table> + IntoUpdateTarget,
{}
