use associations::HasTable;
#[cfg(any(feature = "sqlite", feature = "mysql"))]
use associations::Identifiable;
#[cfg(any(feature = "sqlite", feature = "mysql"))]
use dsl::Find;
#[cfg(any(feature = "sqlite", feature = "postgres", feature = "mysql"))]
use dsl::Update;
use query_builder::{AsChangeset, IntoUpdateTarget};
#[cfg(any(feature = "sqlite", feature = "mysql"))]
use query_dsl::methods::{ExecuteDsl, FindDsl};
#[cfg(any(feature = "sqlite", feature = "postgres", feature = "mysql"))]
use query_dsl::{LoadQuery, RunQueryDsl};
use result::QueryResult;

#[doc(hidden)]
pub trait InternalSaveChangesDsl<T, U> {
    fn internal_save_changes(&self, changeset: T) -> QueryResult<U>;
}

#[cfg(feature = "postgres")]
use pg::PgConnection;

#[cfg(feature = "postgres")]
impl<T, U> InternalSaveChangesDsl<T, U> for PgConnection
where
    T: Copy + AsChangeset<Target = <T as HasTable>::Table> + IntoUpdateTarget,
    Update<T, T>: LoadQuery<PgConnection, U>,
{
    fn internal_save_changes(&self, changeset: T) -> QueryResult<U> {
        ::update(changeset).set(changeset).get_result(self)
    }
}

#[cfg(feature = "sqlite")]
use sqlite::SqliteConnection;

#[cfg(feature = "sqlite")]
impl<T, U> InternalSaveChangesDsl<T, U> for SqliteConnection
where
    T: Copy + Identifiable,
    T: AsChangeset<Target = <T as HasTable>::Table> + IntoUpdateTarget,
    T::Table: FindDsl<T::Id>,
    Update<T, T>: ExecuteDsl<SqliteConnection>,
    Find<T::Table, T::Id>: LoadQuery<SqliteConnection, U>,
{
    fn internal_save_changes(&self, changeset: T) -> QueryResult<U> {
        try!(::update(changeset).set(changeset).execute(self));
        T::table().find(changeset.id()).get_result(self)
    }
}

#[cfg(feature = "mysql")]
use mysql::MysqlConnection;

#[cfg(feature = "mysql")]
impl<T, U> InternalSaveChangesDsl<T, U> for MysqlConnection
where
    T: Copy + Identifiable,
    T: AsChangeset<Target = <T as HasTable>::Table> + IntoUpdateTarget,
    T::Table: FindDsl<T::Id>,
    Update<T, T>: ExecuteDsl<MysqlConnection>,
    Find<T::Table, T::Id>: LoadQuery<MysqlConnection, U>,
{
    fn internal_save_changes(&self, changeset: T) -> QueryResult<U> {
        try!(::update(changeset).set(changeset).execute(self));
        T::table().find(changeset.id()).get_result(self)
    }
}

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
pub trait SaveChangesDsl<Conn>: Sized {
    /// See the trait documentation.
    fn save_changes<T>(self, connection: &Conn) -> QueryResult<T>
    where
        Conn: InternalSaveChangesDsl<Self, T>,
    {
        connection.internal_save_changes(self)
    }
}

impl<T, Conn> SaveChangesDsl<Conn> for T
where
    T: Copy + AsChangeset<Target = <T as HasTable>::Table> + IntoUpdateTarget,
{
}
