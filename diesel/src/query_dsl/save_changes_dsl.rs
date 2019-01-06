use crate::associations::HasTable;
#[cfg(any(feature = "sqlite", feature = "mysql"))]
use crate::associations::Identifiable;
use crate::connection::Connection;
#[cfg(any(feature = "sqlite", feature = "mysql"))]
use crate::dsl::Find;
#[cfg(any(feature = "sqlite", feature = "postgres", feature = "mysql"))]
use crate::dsl::Update;
use crate::query_builder::{AsChangeset, IntoUpdateTarget};
#[cfg(any(feature = "sqlite", feature = "mysql"))]
use crate::query_dsl::methods::{ExecuteDsl, FindDsl};
#[cfg(any(feature = "sqlite", feature = "postgres", feature = "mysql"))]
use crate::query_dsl::{LoadQuery, RunQueryDsl};
use crate::result::QueryResult;

/// A trait defining how to update a record and fetch the updated entry
/// on a certain backend.
///
/// The only case where it is required to work with this trait is while
/// implementing a new connection type.
/// Otherwise use [`SaveChangesDsl`](trait.SaveChangesDsl.html)
///
/// For implementing this trait for a custom backend:
/// * The `Changes` generic parameter represents the changeset that should be stored
/// * The `Output` generic parameter represents the type of the response.
pub trait UpdateAndFetchResults<Changes, Output>: Connection {
    /// See the traits documentation.
    fn update_and_fetch(&self, changeset: Changes) -> QueryResult<Output>;
}

#[cfg(feature = "postgres")]
use crate::pg::PgConnection;

#[cfg(feature = "postgres")]
impl<Changes, Output> UpdateAndFetchResults<Changes, Output> for PgConnection
where
    Changes: Copy + AsChangeset<Target = <Changes as HasTable>::Table> + IntoUpdateTarget,
    Update<Changes, Changes>: LoadQuery<PgConnection, Output>,
{
    fn update_and_fetch(&self, changeset: Changes) -> QueryResult<Output> {
        crate::update(changeset).set(changeset).get_result(self)
    }
}

#[cfg(feature = "sqlite")]
use crate::sqlite::SqliteConnection;

#[cfg(feature = "sqlite")]
impl<Changes, Output> UpdateAndFetchResults<Changes, Output> for SqliteConnection
where
    Changes: Copy + Identifiable,
    Changes: AsChangeset<Target = <Changes as HasTable>::Table> + IntoUpdateTarget,
    Changes::Table: FindDsl<Changes::Id>,
    Update<Changes, Changes>: ExecuteDsl<SqliteConnection>,
    Find<Changes::Table, Changes::Id>: LoadQuery<SqliteConnection, Output>,
{
    fn update_and_fetch(&self, changeset: Changes) -> QueryResult<Output> {
        crate::update(changeset).set(changeset).execute(self)?;
        Changes::table().find(changeset.id()).get_result(self)
    }
}

#[cfg(feature = "mysql")]
use crate::mysql::MysqlConnection;

#[cfg(feature = "mysql")]
impl<Changes, Output> UpdateAndFetchResults<Changes, Output> for MysqlConnection
where
    Changes: Copy + Identifiable,
    Changes: AsChangeset<Target = <Changes as HasTable>::Table> + IntoUpdateTarget,
    Changes::Table: FindDsl<Changes::Id>,
    Update<Changes, Changes>: ExecuteDsl<MysqlConnection>,
    Find<Changes::Table, Changes::Id>: LoadQuery<MysqlConnection, Output>,
{
    fn update_and_fetch(&self, changeset: Changes) -> QueryResult<Output> {
        crate::update(changeset).set(changeset).execute(self)?;
        Changes::table().find(changeset.id()).get_result(self)
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
/// # use self::schema::animals;
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
/// #     use self::animals::dsl::*;
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
pub trait SaveChangesDsl<Conn> {
    /// See the trait documentation.
    fn save_changes<T>(self, connection: &Conn) -> QueryResult<T>
    where
        Self: Sized,
        Conn: UpdateAndFetchResults<Self, T>,
    {
        connection.update_and_fetch(self)
    }
}

impl<T, Conn> SaveChangesDsl<Conn> for T where
    T: Copy + AsChangeset<Target = <T as HasTable>::Table> + IntoUpdateTarget
{
}
