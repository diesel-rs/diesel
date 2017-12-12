use associations::HasTable;
#[cfg(any(feature = "sqlite", feature = "postgres", feature = "mysql"))]
use dsl::Update;
use query_builder::{AsChangeset, IntoUpdateTarget};
#[cfg(any(feature = "sqlite", feature = "postgres", feature = "mysql"))]
use query_dsl::{LoadQuery, RunQueryDsl};
use result::QueryResult;
#[cfg(any(feature = "sqlite", feature = "mysql"))]
use associations::Identifiable;
#[cfg(any(feature = "sqlite", feature = "mysql"))]
use dsl::Find;
#[cfg(any(feature = "sqlite", feature = "mysql"))]
use query_dsl::methods::{ExecuteDsl, FindDsl};

pub trait InternalSaveChangesDsl<Conn, T>: Sized {
    fn internal_save_changes(self, connection: &Conn) -> QueryResult<T>;
}

#[cfg(feature = "postgres")]
use pg::PgConnection;

#[cfg(feature = "postgres")]
impl<T, U> InternalSaveChangesDsl<PgConnection, U> for T
where
    T: Copy + AsChangeset<Target = <T as HasTable>::Table> + IntoUpdateTarget,
    Update<T, T>: LoadQuery<PgConnection, U>,
{
    fn internal_save_changes(self, conn: &PgConnection) -> QueryResult<U> {
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
    /// On backends which support the `RETURNING` keyword,
    /// `foo.save_changes(&conn)` is equivalent to
    /// `update(&foo).set(&foo).get_result(&conn)`.
    /// On other backends, two queries will be executed.
    ///
    /// # Example
    ///
    /// ```rust
    /// # #[macro_use] extern crate diesel;
    /// # include!("../doctest_setup.rs");
    /// # use schema::animals;
    /// #
    /// #[derive(Queryable, Debug, PartialEq)]
    /// struct Animal {
    ///    id: i32,
    ///    species: String,
    ///    legs: i32,
    ///    name: Option<String>,
    /// }
    ///
    /// #[derive(AsChangeset, Identifiable)]
    /// #[table_name = "animals"]
    /// struct AnimalForm<'a> {
    ///     id: i32,
    ///     name: &'a str,
    /// }
    ///
    /// # fn main() {
    /// #     run_test();
    /// # }
    /// #
    /// # fn run_test() -> QueryResult<()> {
    /// #     use animals::dsl::*;
    /// #     let connection = establish_connection();
    /// let form = AnimalForm { id: 2, name: "Super scary" };
    /// let changed_animal = form.save_changes(&connection)?;
    /// let expected_animal = Animal {
    ///     id: 2,
    ///     species: String::from("spider"),
    ///     legs: 8,
    ///     name: Some(String::from("Super scary")),
    /// };
    /// assert_eq!(expected_animal, changed_animal);
    /// #     Ok(())
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
