use associations::HasTable;
use dsl::Update;
use query_builder::{AsChangeset, IntoUpdateTarget};
use query_dsl::{LoadDsl, LoadQuery};
use result::QueryResult;
#[cfg(any(feature = "sqlite", feature = "mysql"))]
use associations::Identifiable;
#[cfg(any(feature = "sqlite", feature = "mysql"))]
use dsl::Find;
#[cfg(any(feature = "sqlite", feature = "mysql"))]
use query_dsl::ExecuteDsl;
#[cfg(any(feature = "sqlite", feature = "mysql"))]
use query_dsl::methods::FindDsl;

pub trait InternalSaveChangesDsl<Conn, T>: Sized {
    fn internal_save_changes(self, connection: &Conn) -> QueryResult<T>;
}

impl<T, U, Conn> InternalSaveChangesDsl<Conn, U> for T
where
    T: Copy + AsChangeset<Target = <T as HasTable>::Table> + IntoUpdateTarget,
    Update<T, T>: LoadDsl<Conn> + LoadQuery<Conn, U>,
{
    fn internal_save_changes(self, conn: &Conn) -> QueryResult<U> {
        ::update(self).set(self).get_result(conn)
    }
}

#[cfg(feature = "sqlite")]
use sqlite::SqliteConnection;

#[cfg(feature = "sqlite")]
impl<T, U> InternalSaveChangesDsl<SqliteConnection, U> for T
where
    T: Copy + Identifiable,
    T: AsChangeset<Target = <T as HasTable>::Table> + IntoUpdateTarget,
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
impl<T, U> InternalSaveChangesDsl<MysqlConnection, U> for T
where
    T: Copy + Identifiable,
    T: AsChangeset<Target = <T as HasTable>::Table> + IntoUpdateTarget,
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
    /// Sugar for types which implement both `AsChangeset` and `Identifiable`
    ///
    /// `foo.save_changes(&conn)` is equivalent to
    /// `update(foo::table().find(foo.id())).set(&foo).get_result(&conn)`
    ///
    /// # Example
    ///
    /// ```rust
    /// # #[macro_use] extern crate diesel;
    /// # include!("../doctest_setup.rs");
    /// # use schema::users;
    /// #
    /// # #[derive(AsChangeset, Debug, PartialEq, Identifiable, Queryable)]
    /// # struct Animal {
    /// #    id: i32,
    /// #    species: String,
    /// #    legs: i32,
    /// #    name: Option<String>,
    /// # }
    /// #
    /// # fn main() {
    /// #     use animals::dsl::*;
    /// #     let connection = establish_connection();
    /// let mut spider = animals.filter(species.eq("spider"))
    ///     .first::<Animal>(&connection)
    ///     .expect("Too scary to load");
    ///
    /// spider.species = String::from("solifuge");
    /// spider.legs = 10;
    ///
    /// spider.save_changes::<Animal>(&connection).expect("Error saving changes");
    ///
    /// let changed_animal = animals.find(spider.id())
    ///     .first::<Animal>(&connection);
    ///
    /// assert_eq!(Ok(Animal { id: 2, species: "solifuge".to_string(), legs: 10, name: None }), changed_animal);
    /// # }
    /// ```
    fn save_changes<T>(self, connection: &Conn) -> QueryResult<T>
    where
        Self: InternalSaveChangesDsl<Conn, T>,
    {
        self.internal_save_changes(connection)
    }
}

impl<T, Conn> SaveChangesDsl<Conn> for T
where
    T: Copy + AsChangeset<Target = <T as HasTable>::Table> + IntoUpdateTarget,
{
}
