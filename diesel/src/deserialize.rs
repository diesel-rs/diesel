//! Types and traits related to deserializing values from the database

use std::error::Error;
use std::result;

use backend::{self, Backend};
use row::{NamedRow, Row};

/// A specialized result type representing the result of deserializing
/// a value from the database.
pub type Result<T> = result::Result<T, Box<Error + Send + Sync>>;

/// Trait indicating that a record can be queried from the database.
///
/// Types which implement `Queryable` represent the result of a SQL query. This
/// does not necessarily mean they represent a single database table.
///
/// Diesel represents the return type of a query as a tuple. The purpose of this
/// trait is to convert from a tuple of Rust values that have been deserialized
/// into your struct.
///
/// # Deriving
///
/// This trait can be derived automatically using `#[derive(Queryable)]`. This
/// trait can only be derived for structs, not enums.
///
/// When this trait is derived, it will assume that the order of fields on your
/// struct match the order of the fields in the query. This means that field
/// order is significant if you are using `#[derive(Queryable)]`. Field name has
/// no effect.
///
/// To provide custom deserialization behavior for a field, you can use
/// `#[diesel(deserialize_as = "Type")]`. If this attribute is present, Diesel
/// will deserialize into that type, rather than the type on your struct and
/// call `.into` to convert it. This can be used to add custom behavior for a
/// single field, or use types that are otherwise unsupported by Diesel.
///
/// # Examples
///
/// If we just want to map a query to our struct, we can use `derive`.
///
/// ```rust
/// # #[macro_use] extern crate diesel;
/// # include!("doctest_setup.rs");
/// #
/// #[derive(Queryable, PartialEq, Debug)]
/// struct User {
///     id: i32,
///     name: String,
/// }
///
/// # fn main() {
/// #     run_test();
/// # }
/// #
/// # fn run_test() -> QueryResult<()> {
/// #     use schema::users::dsl::*;
/// #     let connection = establish_connection();
/// let first_user = users.first(&connection)?;
/// let expected = User { id: 1, name: "Sean".into() };
/// assert_eq!(expected, first_user);
/// #     Ok(())
/// # }
/// ```
///
/// If we want to do additional work during deserialization, we can use
/// `deserialize_as` to use a different implementation.
///
/// ```rust
/// # #[macro_use] extern crate diesel;
/// # include!("doctest_setup.rs");
/// #
/// # use schema::users;
/// # use diesel::backend::{self, Backend};
/// # use diesel::deserialize::Queryable;
/// #
/// struct LowercaseString(String);
///
/// impl Into<String> for LowercaseString {
///     fn into(self) -> String {
///         self.0
///     }
/// }
///
/// impl<DB, ST> Queryable<ST, DB> for LowercaseString
/// where
///     DB: Backend,
///     String: Queryable<ST, DB>,
/// {
///     type Row = <String as Queryable<ST, DB>>::Row;
///
///     fn build(row: Self::Row) -> Self {
///         LowercaseString(String::build(row).to_lowercase())
///     }
/// }
///
/// #[derive(Queryable, PartialEq, Debug)]
/// struct User {
///     id: i32,
///     #[diesel(deserialize_as = "LowercaseString")]
///     name: String,
/// }
///
/// # fn main() {
/// #     run_test();
/// # }
/// #
/// # fn run_test() -> QueryResult<()> {
/// #     use schema::users::dsl::*;
/// #     let connection = establish_connection();
/// let first_user = users.first(&connection)?;
/// let expected = User { id: 1, name: "sean".into() };
/// assert_eq!(expected, first_user);
/// #     Ok(())
/// # }
/// ```
///
/// Alternatively, we can implement the trait for our struct manually.
///
/// ```rust
/// # #[macro_use] extern crate diesel;
/// # include!("doctest_setup.rs");
/// #
/// use schema::users;
/// use diesel::deserialize::Queryable;
///
/// # /*
/// type DB = diesel::sqlite::Sqlite;
/// # */
///
/// #[derive(PartialEq, Debug)]
/// struct User {
///     id: i32,
///     name: String,
/// }
///
/// impl Queryable<users::SqlType, DB> for User {
///     type Row = (i32, String);
///
///     fn build(row: Self::Row) -> Self {
///         User {
///             id: row.0,
///             name: row.1.to_lowercase(),
///         }
///     }
/// }
///
/// # fn main() {
/// #     run_test();
/// # }
/// #
/// # fn run_test() -> QueryResult<()> {
/// #     use schema::users::dsl::*;
/// #     let connection = establish_connection();
/// let first_user = users.first(&connection)?;
/// let expected = User { id: 1, name: "sean".into() };
/// assert_eq!(expected, first_user);
/// #     Ok(())
/// # }
/// ```
pub trait Queryable<ST, DB>
where
    DB: Backend,
{
    /// The Rust type you'd like to map from.
    ///
    /// This is typically a tuple of all of your struct's fields.
    type Row: FromSqlRow<ST, DB>;

    /// Construct an instance of this type
    fn build(row: Self::Row) -> Self;
}

/// Deserializes the result of a query constructed with [`sql_query`].
///
/// # Deriving
///
/// To derive this trait, Diesel needs to know the SQL type of each field. You
/// can do this by either annotating your struct with `#[table_name =
/// "some_table"]` (in which case the SQL type will be
/// `diesel::dsl::SqlTypeOf<table_name::column_name>`), or by annotating each
/// field with `#[sql_type = "SomeType"]`.
///
/// If you are using `#[table_name]`, the module for that table must be in
/// scope. For example, to derive this for a struct called `User`, you will
/// likely need a line such as `use schema::users;`
///
/// If the name of a field on your struct is different than the column in your
/// `table!` declaration, or if you are deriving this trait on a tuple struct,
/// you can annotate the field with `#[column_name = "some_column"]`. For tuple
/// structs, all fields must have this annotation.
///
/// If a field is another struct which implements `QueryableByName`, instead of
/// a column, you can annotate that struct with `#[diesel(embed)]`
///
/// To provide custom deserialization behavior for a field, you can use
/// `#[diesel(deserialize_as = "Type")]`. If this attribute is present, Diesel
/// will deserialize into that type, rather than the type on your struct and
/// call `.into` to convert it. This can be used to add custom behavior for a
/// single field, or use types that are otherwise unsupported by Diesel.
///
/// [`sql_query`]: ../fn.sql_query.html
///
/// # Examples
///
///
/// If we just want to map a query to our struct, we can use `derive`.
///
/// ```rust
/// # #[macro_use] extern crate diesel;
/// # include!("doctest_setup.rs");
/// # use schema::users;
/// # use diesel::sql_query;
/// #
/// #[derive(QueryableByName, PartialEq, Debug)]
/// #[table_name = "users"]
/// struct User {
///     id: i32,
///     name: String,
/// }
///
/// # fn main() {
/// #     run_test();
/// # }
/// #
/// # fn run_test() -> QueryResult<()> {
/// #     let connection = establish_connection();
/// let first_user = sql_query("SELECT * FROM users ORDER BY id LIMIT 1")
///     .get_result(&connection)?;
/// let expected = User { id: 1, name: "Sean".into() };
/// assert_eq!(expected, first_user);
/// #     Ok(())
/// # }
/// ```
///
/// If we want to do additional work during deserialization, we can use
/// `deserialize_as` to use a different implementation.
///
/// ```rust
/// # #[macro_use] extern crate diesel;
/// # include!("doctest_setup.rs");
/// # use diesel::sql_query;
/// # use schema::users;
/// # use diesel::backend::{self, Backend};
/// # use diesel::deserialize::{self, FromSql};
/// #
/// struct LowercaseString(String);
///
/// impl Into<String> for LowercaseString {
///     fn into(self) -> String {
///         self.0
///     }
/// }
///
/// impl<DB, ST> FromSql<ST, DB> for LowercaseString
/// where
///     DB: Backend,
///     String: FromSql<ST, DB>,
/// {
///     fn from_sql(bytes: Option<backend::RawValue<DB>>) -> deserialize::Result<Self> {
///         String::from_sql(bytes)
///             .map(|s| LowercaseString(s.to_lowercase()))
///     }
/// }
///
/// #[derive(QueryableByName, PartialEq, Debug)]
/// #[table_name = "users"]
/// struct User {
///     id: i32,
///     #[diesel(deserialize_as = "LowercaseString")]
///     name: String,
/// }
///
/// # fn main() {
/// #     run_test();
/// # }
/// #
/// # fn run_test() -> QueryResult<()> {
/// #     let connection = establish_connection();
/// let first_user = sql_query("SELECT * FROM users ORDER BY id LIMIT 1")
///     .get_result(&connection)?;
/// let expected = User { id: 1, name: "sean".into() };
/// assert_eq!(expected, first_user);
/// #     Ok(())
/// # }
/// ```
pub trait QueryableByName<DB>
where
    Self: Sized,
    DB: Backend,
{
    /// Construct an instance of `Self` from the database row
    fn build<R: NamedRow<DB>>(row: &R) -> Result<Self>;
}

/// Deserialize a single field of a given SQL type.
///
/// When possible, implementations of this trait should prefer to use an
/// existing implementation, rather than reading from `bytes`. (For example, if
/// you are implementing this for an enum which is represented as an integer in
/// the database, prefer `i32::from_sql(bytes)` over reading from `bytes`
/// directly)
///
/// Types which implement this trait should also have `#[derive(FromSqlRow)]`
///
/// ### Backend specific details
///
/// - For PostgreSQL, the bytes will be sent using the binary protocol, not text.
/// - For SQLite, the actual type of `DB::RawValue` is private API. All
///   implementations of this trait must be written in terms of an existing
///   primitive.
/// - For MySQL, the value of `bytes` will depend on the return value of
///   `type_metadata` for the given SQL type. See [`MysqlType`] for details.
/// - For third party backends, consult that backend's documentation.
///
/// [`MysqlType`]: ../mysql/enum.MysqlType.html
///
/// ### Examples
///
/// Most implementations of this trait will be defined in terms of an existing
/// implementation.
///
/// ```rust
/// # use diesel::backend::{self, Backend};
/// # use diesel::sql_types::*;
/// # use diesel::deserialize::{self, FromSql};
/// #
/// #[repr(i32)]
/// #[derive(Debug, Clone, Copy)]
/// pub enum MyEnum {
///     A = 1,
///     B = 2,
/// }
///
/// impl<DB> FromSql<Integer, DB> for MyEnum
/// where
///     DB: Backend,
///     i32: FromSql<Integer, DB>,
/// {
///     fn from_sql(bytes: Option<backend::RawValue<DB>>) -> deserialize::Result<Self> {
///         match i32::from_sql(bytes)? {
///             1 => Ok(MyEnum::A),
///             2 => Ok(MyEnum::B),
///             x => Err(format!("Unrecognized variant {}", x).into()),
///         }
///     }
/// }
/// ```
pub trait FromSql<A, DB: Backend>: Sized {
    /// See the trait documentation.
    fn from_sql(bytes: Option<backend::RawValue<DB>>) -> Result<Self>;
}

/// Deserialize one or more fields.
///
/// All types which implement `FromSql` should also implement this trait. This
/// trait differs from `FromSql` in that it is also implemented by tuples.
/// Implementations of this trait are usually derived.
///
/// In the future, we hope to be able to provide a blanket impl of this trait
/// for all types which implement `FromSql`. However, as of Diesel 1.0, such an
/// impl would conflict with our impl for tuples.
///
/// ## Deriving
///
/// This trait can be automatically derived by Diesel
/// for any type which implements `FromSql`.
/// There are no options or special considerations needed for this derive.
/// Note that `#[derive(FromSqlRow)]` will also generate a `Queryable` implementation.
pub trait FromSqlRow<A, DB: Backend>: Sized {
    /// The number of fields that this type will consume. Must be equal to
    /// the number of times you would call `row.take()` in `build_from_row`
    const FIELDS_NEEDED: usize = 1;

    /// See the trait documentation.
    fn build_from_row<T: Row<DB>>(row: &mut T) -> Result<Self>;
}

// Reasons we can't write this:
//
// impl<T, ST, DB> FromSqlRow<ST, DB> for T
// where
//     DB: Backend + HasSqlType<ST>,
//     T: FromSql<ST, DB>,
// {
//     fn build_from_row<T: Row<DB>>(row: &mut T) -> Result<Self> {
//         Self::from_sql(row.take())
//     }
// }
//
// (this is mostly here so @sgrif has a better reference every time he thinks
// he's somehow had a breakthrough on solving this problem):
//
// - It conflicts with our impl for tuples, because `DB` is a bare type
//   parameter, it could in theory be a local type for some other impl.
//   - This is fixed by replacing our impl with 3 impls, where `DB` is changed
//     concrete backends. This would mean that any third party crates adding new
//     backends would need to add the tuple impls, which sucks but is fine.
// - It conflicts with our impl for `Option`
//   - So we could in theory fix this by both splitting the generic impl into
//     backend specific impls, and removing the `FromSql` impls. In theory there
//     is no reason that it needs to implement `FromSql`, since everything
//     requires `FromSqlRow`, but it really feels like it should.
//   - Specialization might also fix this one. The impl isn't quite a strict
//     subset (the `FromSql` impl has `T: FromSql`, and the `FromSqlRow` impl
//     has `T: FromSqlRow`), but if `FromSql` implies `FromSqlRow`,
//     specialization might consider that a subset?
// - I don't know that we really need it. `#[derive(FromSqlRow)]` is probably
//   good enough. That won't improve our own codebase, since 99% of our
//   `FromSqlRow` impls are for types from another crate, but it's almost
//   certainly good enough for user types.
//   - Still, it really feels like `FromSql` *should* be able to imply both
//   `FromSqlRow` and `Queryable`
