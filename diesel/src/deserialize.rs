//! Types and traits related to deserializing values from the database

use std::error::Error;
use std::result;

use crate::backend::{self, Backend};
use crate::row::{NamedRow, Row, Field};
use crate::expression::select_by::SelectBy;
use crate::sql_types::{SingleValue, SqlType, Untyped};
use crate::Selectable;

/// A specialized result type representing the result of deserializing
/// a value from the database.
pub type Result<T> = result::Result<T, Box<dyn Error + Send + Sync>>;

/// Trait indicating that a record can be queried from the database.
///
/// Types which implement `Queryable` represent the result of a SQL query. This
/// does not necessarily mean they represent a single database table.
///
/// Diesel represents the return type of a query as a tuple. The purpose of this
/// trait is to convert from a tuple of Rust values that have been deserialized
/// into your struct.
///
/// This trait can be [derived](derive@Queryable)
///
/// ## Interaction with `NULL`/`Option`
/// [`Nullable`][crate::sql_types::Nullable] types can be queried into `Option`.
/// This is valid for single fields, tuples, and structures with `Queryable`.
///
/// With tuples and structs, the process for deserializing an `Option<(A,B,C)>` is
/// to attempt to deserialize `A`, `B` and `C`, and if either of these return an
/// [`UnexpectedNullError`](crate::result::UnexpectedNullError), the `Option` will be
/// deserialized as `None`.  
/// If all succeed, the `Option` will be deserialized as `Some((a,b,c))`.
///
/// # Examples
///
/// ## Simple mapping from query to struct
///
/// If we just want to map a query to our struct, we can use `derive`.
///
/// ```rust
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
/// #     let connection = &mut establish_connection();
/// let first_user = users.order_by(id).first(connection)?;
/// let expected = User { id: 1, name: "Sean".into() };
/// assert_eq!(expected, first_user);
/// #     Ok(())
/// # }
/// ```
///
/// ## Interaction with `NULL`/`Option`
///
/// ### Single field
/// ```rust
/// # include!("doctest_setup.rs");
/// # use diesel::sql_types::*;
/// #
/// table! {
///     animals {
///         id -> Integer,
///         species -> VarChar,
///         legs -> Integer,
///         name -> Nullable<VarChar>,
///     }
/// }
/// #
/// #[derive(Queryable, PartialEq, Debug)]
/// struct Animal {
///     id: i32,
///     name: Option<String>,
/// }
///
/// # fn main() {
/// #     run_test();
/// # }
/// #
/// # fn run_test() -> QueryResult<()> {
/// #     use schema::animals::dsl::*;
/// #     let connection = &mut establish_connection();
/// let all_animals = animals.select((id, name)).order_by(id).load(connection)?;
/// let expected = vec![Animal { id: 1, name: Some("Jack".to_owned()) }, Animal { id: 2, name: None }];
/// assert_eq!(expected, all_animals);
/// #     Ok(())
/// # }
/// ```
///
/// ### Multiple fields
/// ```rust
/// # include!("doctest_setup.rs");
/// #
/// #[derive(Queryable, PartialEq, Debug)]
/// struct UserWithPost {
///     id: i32,
///     post: Option<Post>,
/// }
/// #[derive(Queryable, PartialEq, Debug)]
/// struct Post {
///     id: i32,
///     title: String,
/// }
///
/// # fn main() {
/// #     run_test();
/// # }
/// #
/// # fn run_test() -> QueryResult<()> {
/// #     use schema::{posts, users};
/// #     let connection = &mut establish_connection();
/// #     diesel::insert_into(users::table)
/// #         .values(users::name.eq("Ruby"))
/// #         .execute(connection)?;
/// let all_posts = users::table
///     .left_join(posts::table)
///     .select((
///         users::id,
///         (posts::id, posts::title).nullable()
///     ))
///     .order_by((users::id, posts::id))
///     .load(connection)?;
/// let expected = vec![
///     UserWithPost { id: 1, post: Some(Post { id: 1, title: "My first post".to_owned() }) },
///     UserWithPost { id: 1, post: Some(Post { id: 2, title: "About Rust".to_owned() }) },
///     UserWithPost { id: 2, post: Some(Post { id: 3, title: "My first post too".to_owned() }) },
///     UserWithPost { id: 3, post: None },
/// ];
/// assert_eq!(expected, all_posts);
/// #     Ok(())
/// # }
/// ```
///
/// ## `deserialize_as` attribute
///
/// If we want to do additional work during deserialization, we can use
/// `deserialize_as` to use a different implementation.
///
/// ```rust
/// # include!("doctest_setup.rs");
/// #
/// # use schema::users;
/// # use diesel::backend::{self, Backend};
/// # use diesel::deserialize::{self, Queryable, FromSql};
/// # use diesel::sql_types::Text;
/// #
/// struct LowercaseString(String);
///
/// impl Into<String> for LowercaseString {
///     fn into(self) -> String {
///         self.0
///     }
/// }
///
/// impl<DB> Queryable<Text, DB> for LowercaseString
/// where
///     DB: Backend,
///     String: FromSql<Text, DB>,
/// {
///     type Row = String;
///
///     fn build(s: String) -> deserialize::Result<Self> {
///         Ok(LowercaseString(s.to_lowercase()))
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
/// #     let connection = &mut establish_connection();
/// let first_user = users.first(connection)?;
/// let expected = User { id: 1, name: "sean".into() };
/// assert_eq!(expected, first_user);
/// #     Ok(())
/// # }
/// ```
///
/// ## Manual implementation
///
/// Alternatively, we can implement the trait for our struct manually.
///
/// ```rust
/// # include!("doctest_setup.rs");
/// #
/// use schema::users;
/// use diesel::deserialize::{self, Queryable};
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
///     fn build(row: Self::Row) -> deserialize::Result<Self> {
///         Ok(User {
///             id: row.0,
///             name: row.1.to_lowercase(),
///         })
///     }
/// }
///
/// # fn main() {
/// #     run_test();
/// # }
/// #
/// # fn run_test() -> QueryResult<()> {
/// #     use schema::users::dsl::*;
/// #     let connection = &mut establish_connection();
/// let first_user = users.first(connection)?;
/// let expected = User { id: 1, name: "sean".into() };
/// assert_eq!(expected, first_user);
/// #     Ok(())
/// # }
/// ```
pub trait Queryable<ST, DB>: Sized
where
    DB: Backend,
{
    /// The Rust type you'd like to map from.
    ///
    /// This is typically a tuple of all of your struct's fields.
    type Row: FromStaticSqlRow<ST, DB>;

    /// Construct an instance of this type
    fn build(row: Self::Row) -> Result<Self>;
}

#[doc(inline)]
pub use diesel_derives::Queryable;

/// Deserializes the result of a query constructed with [`sql_query`].
///
/// This trait can be [derived](derive@QueryableByName)
///
/// [`sql_query`]: crate::sql_query()
///
/// # Examples
///
/// If we just want to map a query to our struct, we can use `derive`.
///
/// ```rust
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
/// #     let connection = &mut establish_connection();
/// let first_user = sql_query("SELECT * FROM users ORDER BY id LIMIT 1")
///     .get_result(connection)?;
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
///     fn from_sql(bytes: backend::RawValue<DB>) -> deserialize::Result<Self> {
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
/// #     let connection = &mut establish_connection();
/// let first_user = sql_query("SELECT * FROM users ORDER BY id LIMIT 1")
///     .get_result(connection)?;
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
    fn build<'a>(row: &impl NamedRow<'a, DB>) -> Result<Self>;
}

#[doc(inline)]
pub use diesel_derives::QueryableByName;

/// Deserialize a single field of a given SQL type.
///
/// When possible, implementations of this trait should prefer to use an
/// existing implementation, rather than reading from `bytes`. (For example, if
/// you are implementing this for an enum which is represented as an integer in
/// the database, prefer `i32::from_sql(bytes)` (or the explicit form
/// `<i32 as FromSql<Integer, DB>>::from_sql(bytes)`) over reading from `bytes`
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
/// # use diesel::deserialize::{self, FromSql, FromSqlRow};
/// #
/// #[repr(i32)]
/// #[derive(Debug, Clone, Copy, FromSqlRow)]
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
///     fn from_sql(bytes: backend::RawValue<DB>) -> deserialize::Result<Self> {
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
    fn from_sql(bytes: backend::RawValue<DB>) -> Result<Self>;

    /// A specialized variant of `from_sql` for handling null values.
    ///
    /// The default implementation returns an `UnexpectedNullError` for
    /// an encountered null value and calls `Self::from_sql` otherwise
    ///
    /// If your custom type supports null values you need to provide a
    /// custom implementation.
    #[inline(always)]
    fn from_nullable_sql(bytes: Option<backend::RawValue<DB>>) -> Result<Self> {
        match bytes {
            Some(bytes) => Self::from_sql(bytes),
            None => {
                println!("from_nullable_sql");
                Err(Box::new(crate::result::UnexpectedNullError))}
        }
    }

    /// A specialized variant of `from_sql` for handling null values.
    ///
    /// The default implementation returns an `UnexpectedNullError` for
    /// an encountered null value and calls `Self::from_sql` otherwise
    ///
    /// If your custom type supports null values you need to provide a
    /// custom implementation.
    #[inline(always)]
    fn from_nullable_sql_field<'a>(field: &dyn Field<'a, DB>) -> Result<Self> {
        let bytes = field.value();
        match bytes {
            Some(bytes) => {
                Self::from_sql(bytes)
            },
            None => {
                if let Some(field_name) = field.field_name(){
                    println!("field: {} value is None, but field is not Option<Type>", field_name);
                }
                else{
                    println!("field:(none name) value is None, but field is not Option<Type>");
                }
                Err(Box::new(crate::result::UnexpectedNullError))}
        }
    }

}

/// Deserialize a database row into a rust data structure
///
/// Diesel provides wild card implementations of this trait for all types
/// that implement one of the following traits:
///    * [`Queryable`]
///    * [`QueryableByName`]
pub trait FromSqlRow<ST, DB: Backend>: Sized {
    /// See the trait documentation.
    fn build_from_row<'a>(row: &impl Row<'a, DB>) -> Result<Self>;
}

#[doc(inline)]
pub use diesel_derives::FromSqlRow;

/// A marker trait indicating that the corresponding type consumes a static at
/// compile time known number of field
///
/// There is normally no need to implement this trait. Diesel provides
/// wild card impls for all types that implement `FromSql<ST, DB>` or `Queryable<ST, DB>`
/// where the size of `ST` is known
pub trait StaticallySizedRow<ST, DB: Backend>: FromSqlRow<ST, DB> {
    /// The number of fields that this type will consume.
    const FIELD_COUNT: usize;
}

impl<DB, T> FromSqlRow<Untyped, DB> for T
where
    DB: Backend,
    T: QueryableByName<DB>,
{
    fn build_from_row<'a>(row: &impl Row<'a, DB>) -> Result<Self> {
        T::build(row)
    }
}

/// A helper trait to deserialize a statically sized row into an tuple
///
/// **If you see an error message mentioning this trait you likly
///   trying to map the result of an query to an struct with missmatching
///   field types. Recheck your field order and the concrete field types**
///
/// You should not need to implement this trait directly.
/// Diesel provides wild card implementations for any supported tuple size
/// and for any type that implements `FromSql<ST, DB>`.
///
// This is a distinct trait from `FromSqlRow` because otherwise we
// are getting conflicting implementation errors for our `FromSqlRow`
// implementation for tuples and our wild card impl for all types
// implementing `Queryable`
pub trait FromStaticSqlRow<ST, DB: Backend>: Sized {
    /// See the trait documentation
    fn build_from_row<'a>(row: &impl Row<'a, DB>) -> Result<Self>;
}

#[doc(hidden)]
pub trait SqlTypeOrSelectable {}

impl<ST> SqlTypeOrSelectable for ST where ST: SqlType + SingleValue {}
impl<U, DB> SqlTypeOrSelectable for SelectBy<U, DB>
where
    U: Selectable<DB>,
    DB: Backend,
{
}

impl<T, ST, DB> FromSqlRow<ST, DB> for T
where
    T: Queryable<ST, DB>,
    ST: SqlTypeOrSelectable,
    DB: Backend,
    T::Row: FromStaticSqlRow<ST, DB>,
{
    fn build_from_row<'a>(row: &impl Row<'a, DB>) -> Result<Self> {
        let row = <T::Row as FromStaticSqlRow<ST, DB>>::build_from_row(row)?;
        T::build(row)
    }
}

impl<T, ST, DB> FromStaticSqlRow<ST, DB> for T
where
    DB: Backend,
    T: FromSql<ST, DB>,
    ST: SingleValue,
{
    fn build_from_row<'a>(row: &impl Row<'a, DB>) -> Result<Self> {
        let field = row.get(0).ok_or(crate::result::UnexpectedEndOfRow)?;

        T::from_nullable_sql_field(&field)
    }
}

// We cannot have this impl because rustc
// then complains in third party crates that
// diesel may implement `SingleValue` for tuples
// in the future. While that is theoretically true,
// that will likly not happen in practice.
// If we get negative trait impls at some point in time
// it should be possible to make this work.
/*impl<T, ST, DB> Queryable<ST, DB> for T
where
    DB: Backend,
    T: FromStaticSqlRow<ST, DB>,
    ST: SingleValue,
{
    type Row = Self;

    fn build(row: Self::Row) -> Self {
        row;
    }
}*/

impl<T, ST, DB> StaticallySizedRow<ST, DB> for T
where
    ST: SqlTypeOrSelectable + crate::util::TupleSize,
    T: Queryable<ST, DB>,
    DB: Backend,
{
    const FIELD_COUNT: usize = <ST as crate::util::TupleSize>::SIZE;
}
