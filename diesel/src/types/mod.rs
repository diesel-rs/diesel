//! Types which represent a SQL data type.
//!
//! The structs in this module are *only* used as markers to represent a SQL type.
//! They should never be used in your structs.
//! If you'd like to know the rust types which can be used for a given SQL type,
//! see the documentation for that SQL type.
//! Additional types may be provided by other crates.
//!
//! To see which SQL type can be used with a given Rust type,
//! see the "Implementors" section of [`FromSql`].
//!
//! [`FromSql`]: trait.FromSql.html
//!
//! Any backend specific types are re-exported through this module

pub mod ops;
mod ord;
#[macro_use]
#[doc(hidden)]
pub mod impls;
mod fold;

use std::fmt;
use std::ops::{Deref, DerefMut};

#[doc(hidden)]
pub mod structs {
    pub mod data_types {
        //! Structs to represent the primitive equivalent of SQL types where
        //! there is no existing Rust primitive, or where using it would be
        //! confusing (such as date and time types). This module will re-export
        //! all backend specific data structures when compiled against that
        //! backend.
        #[cfg(feature = "postgres")]
        pub use pg::data_types::*;
    }
}

pub use self::ord::SqlOrd;
pub use self::fold::Foldable;

use backend::{Backend, TypeMetadata};
use row::Row;
use std::error::Error;
use std::io::{self, Write};

/// The boolean SQL type.
///
/// On backends without a native boolean type,
/// this is emulated with the smallest supported integer.
///
/// ### [`ToSql`](trait.ToSql.html) impls
///
/// - [`bool`][bool]
///
/// ### [`FromSql`](trait.FromSql.html) impls
///
/// - [`bool`][bool]
///
/// [bool]: https://doc.rust-lang.org/nightly/std/primitive.bool.html
#[derive(Debug, Clone, Copy, Default, QueryId, SqlType)]
#[postgres(oid = "16", array_oid = "1000")]
#[sqlite_type = "Integer"]
#[mysql_type = "Tiny"]
pub struct Bool;

/// The tiny integer SQL type.
///
/// This is only available on MySQL.
/// Keep in mind that `infer_schema!` will see `TINYINT(1)` as `Bool`,
/// not `Tinyint`.
///
/// ### [`ToSql`](trait.ToSql.html) impls
///
/// - [`i8`][i8]
///
/// ### [`FromSql`](trait.FromSql.html) impls
///
/// - [`i8`][i8]
///
/// [i8]: https://doc.rust-lang.org/nightly/std/primitive.i8.html
#[derive(Debug, Clone, Copy, Default, QueryId, SqlType)]
#[mysql_type = "Tiny"]
pub struct Tinyint;

/// The small integer SQL type.
///
/// ### [`ToSql`](trait.ToSql.html) impls
///
/// - [`i16`][i16]
///
/// ### [`FromSql`](trait.FromSql.html) impls
///
/// - [`i16`][i16]
///
/// [i16]: https://doc.rust-lang.org/nightly/std/primitive.i16.html
#[derive(Debug, Clone, Copy, Default, QueryId, SqlType)]
#[postgres(oid = "21", array_oid = "1005")]
#[sqlite_type = "SmallInt"]
#[mysql_type = "Short"]
pub struct SmallInt;
#[doc(hidden)]
pub type Int2 = SmallInt;
#[doc(hidden)]
pub type Smallint = SmallInt;

/// The integer SQL type.
///
/// ### [`ToSql`](trait.ToSql.html) impls
///
/// - [`i32`][i32]
///
/// ### [`FromSql`](trait.FromSql.html) impls
///
/// - [`i32`][i32]
///
/// [i32]: https://doc.rust-lang.org/nightly/std/primitive.i32.html
#[derive(Debug, Clone, Copy, Default, QueryId, SqlType)]
#[postgres(oid = "23", array_oid = "1007")]
#[sqlite_type = "Integer"]
#[mysql_type = "Long"]
pub struct Integer;
#[doc(hidden)]
pub type Int4 = Integer;

/// The big integer SQL type.
///
/// ### [`ToSql`](trait.ToSql.html) impls
///
/// - [`i64`][i64]
///
/// ### [`FromSql`](trait.FromSql.html) impls
///
/// - [`i64`][i64]
///
/// [i64]: https://doc.rust-lang.org/nightly/std/primitive.i64.html
#[derive(Debug, Clone, Copy, Default, QueryId, SqlType)]
#[postgres(oid = "20", array_oid = "1016")]
#[sqlite_type = "Long"]
#[mysql_type = "LongLong"]
pub struct BigInt;
#[doc(hidden)]
pub type Int8 = BigInt;
#[doc(hidden)]
pub type Bigint = BigInt;

/// The float SQL type.
///
/// ### [`ToSql`](trait.ToSql.html) impls
///
/// - [`f32`][f32]
///
/// ### [`FromSql`](trait.FromSql.html) impls
///
/// - [`f32`][f32]
///
/// [f32]: https://doc.rust-lang.org/nightly/std/primitive.f32.html
#[derive(Debug, Clone, Copy, Default, QueryId, SqlType)]
#[postgres(oid = "700", array_oid = "1021")]
#[sqlite_type = "Float"]
#[mysql_type = "Float"]
pub struct Float;
#[doc(hidden)]
pub type Float4 = Float;

/// The double precision float SQL type.
///
/// ### [`ToSql`](trait.ToSql.html) impls
///
/// - [`f64`][f64]
///
/// ### [`FromSql`](trait.FromSql.html) impls
///
/// - [`f64`][f64]
///
/// [f64]: https://doc.rust-lang.org/nightly/std/primitive.f64.html
#[derive(Debug, Clone, Copy, Default, QueryId, SqlType)]
#[postgres(oid = "701", array_oid = "1022")]
#[sqlite_type = "Double"]
#[mysql_type = "Double"]
pub struct Double;
#[doc(hidden)]
pub type Float8 = Double;

/// The arbitrary precision numeric SQL type.
///
/// This type is only supported on PostgreSQL and MySQL.
/// On SQLite, [`Double`](struct.Double.html) should be used instead.
///
/// ### [`ToSql`](trait.ToSql.html) impls
///
/// - [`bigdecimal::BigDecimal`] with `feature = ["numeric"]`
///
/// ### [`FromSql`](trait.FromSql.html) impls
///
/// - [`bigdecimal::BigDecimal`] with `feature = ["numeric"]`
///
/// [`bigdecimal::BigDecimal`]: /bigdecimal/struct.BigDecimal.html
#[derive(Debug, Clone, Copy, Default, QueryId, SqlType)]
#[postgres(oid = "1700", array_oid = "1231")]
#[mysql_type = "String"]
pub struct Numeric;

/// Alias for `Numeric`
pub type Decimal = Numeric;

/// The text SQL type.
///
/// On all backends strings must be valid UTF-8.
/// On PostgreSQL strings must not include nul bytes.
///
/// Schema inference will treat all variants of `TEXT` as this type (e.g.
/// `VARCHAR`, `MEDIUMTEXT`, etc).
///
/// ### [`ToSql`](trait.ToSql.html) impls
///
/// - [`String`][String]
/// - [`&str`][str]
///
/// ### [`FromSql`](trait.FromSql.html) impls
///
/// - [`String`][String]
///
/// [String]: https://doc.rust-lang.org/nightly/std/string/struct.String.html
/// [str]: https://doc.rust-lang.org/nightly/std/primitive.str.html
#[derive(Debug, Clone, Copy, Default, QueryId, SqlType)]
#[postgres(oid = "25", array_oid = "1009")]
#[sqlite_type = "Text"]
#[mysql_type = "String"]
pub struct Text;

/// The SQL `VARCHAR` type
///
/// This type is generally interchangeable with `TEXT`, so Diesel has this as an
/// alias rather than a separate type (Diesel does not currently support
/// implicit coercions).
///
/// One notable exception to this is with arrays on PG. `TEXT[]` cannot be
/// coerced to `VARCHAR[]`.  It is recommended that you always use `TEXT[]` if
/// you need a string array on PG.
pub type VarChar = Text;
#[doc(hidden)]
pub type Varchar = VarChar;
#[doc(hidden)]
pub type Char = Text;
#[doc(hidden)]
pub type Tinytext = Text;
#[doc(hidden)]
pub type Mediumtext = Text;
#[doc(hidden)]
pub type Longtext = Text;

/// The binary SQL type.
///
/// Schema inference will treat all variants of `BLOB` as this type (e.g.
/// `VARBINARY`, `MEDIUMBLOB`, etc).
///
/// ### [`ToSql`](trait.ToSql.html) impls
///
/// - [`Vec<u8>`][Vec]
/// - [`&[u8]`][slice]
///
/// ### [`FromSql`](trait.FromSql.html) impls
///
/// - [`Vec<u8>`][Vec]
///
/// [Vec]: https://doc.rust-lang.org/nightly/std/vec/struct.Vec.html
/// [slice]: https://doc.rust-lang.org/nightly/std/primitive.slice.html
#[derive(Debug, Clone, Copy, Default, QueryId, SqlType)]
#[postgres(oid = "17", array_oid = "1001")]
#[sqlite_type = "Binary"]
#[mysql_type = "Blob"]
pub struct Binary;

#[doc(hidden)]
pub type Tinyblob = Binary;
#[doc(hidden)]
pub type Blob = Binary;
#[doc(hidden)]
pub type Mediumblob = Binary;
#[doc(hidden)]
pub type Longblob = Binary;
#[doc(hidden)]
pub type Varbinary = Binary;
#[doc(hidden)]
pub type Bit = Binary;

/// The date SQL type.
///
/// ### [`ToSql`](trait.ToSql.html) impls
///
/// - [`chrono::NaiveDate`][NaiveDate] with `feature = "chrono"`
///
/// ### [`FromSql`](trait.FromSql.html) impls
///
/// - [`chrono::NaiveDate`][NaiveDate] with `feature = "chrono"`
///
/// [NaiveDate]: /chrono/naive/date/struct.NaiveDate.html
#[derive(Debug, Clone, Copy, Default, QueryId, SqlType)]
#[postgres(oid = "1082", array_oid = "1182")]
#[sqlite_type = "Text"]
#[mysql_type = "Date"]
pub struct Date;

/// The interval SQL type.
///
/// This type is currently only implemented for PostgreSQL.
///
/// ### [`ToSql`](trait.ToSql.html) impls
///
/// - [`PgInterval`] which can be constructed using [`IntervalDsl`]
///
/// ### [`FromSql`](trait.FromSql.html) impls
///
/// - [`PgInterval`] which can be constructed using [`IntervalDsl`]
///
/// [`PgInterval`]: ../pg/data_types/struct.PgInterval.html
/// [`IntervalDsl`]: ../pg/expression/extensions/trait.IntervalDsl.html
#[derive(Debug, Clone, Copy, Default, QueryId, SqlType)]
#[postgres(oid = "1186", array_oid = "1187")]
pub struct Interval;

/// The time SQL type.
///
/// ### [`ToSql`](trait.ToSql.html) impls
///
/// - [`chrono::NaiveTime`][NaiveTime] with `feature = "chrono"`
///
/// ### [`FromSql`](trait.FromSql.html) impls
///
/// - [`chrono::NaiveTime`][NaiveTime] with `feature = "chrono"`
///
/// [NaiveTime]: /chrono/naive/time/struct.NaiveTime.html
#[derive(Debug, Clone, Copy, Default, QueryId, SqlType)]
#[postgres(oid = "1083", array_oid = "1183")]
#[sqlite_type = "Text"]
#[mysql_type = "Time"]
pub struct Time;

/// The timestamp SQL type.
///
/// ### [`ToSql`](trait.ToSql.html) impls
///
/// - [`std::time::SystemTime`][SystemTime] (PG only)
/// - [`chrono::NaiveDateTime`][NaiveDateTime] with `feature = "chrono"`
/// - [`time::Timespec`][Timespec] with `feature = "deprecated-time"` (PG only)
///
/// ### [`FromSql`](trait.FromSql.html) impls
///
/// - [`std::time::SystemTime`][SystemTime] (PG only)
/// - [`chrono::NaiveDateTime`][NaiveDateTime] with `feature = "chrono"`
/// - [`time::Timespec`][Timespec] with `feature = "deprecated-time"` (PG only)
///
/// [SystemTime]: https://doc.rust-lang.org/nightly/std/time/struct.SystemTime.html
/// [NaiveDateTime]: /chrono/naive/datetime/struct.NaiveDateTime.html
/// [Timespec]: /time/struct.Timespec.html
#[derive(Debug, Clone, Copy, Default, QueryId, SqlType)]
#[postgres(oid = "1114", array_oid = "1115")]
#[sqlite_type = "Text"]
#[mysql_type = "Timestamp"]
pub struct Timestamp;

/// The nullable SQL type.
///
/// This wraps another SQL type to indicate that it can be null.
/// By default all values are assumed to be `NOT NULL`.
///
/// ### [`ToSql`](trait.ToSql.html) impls
///
/// - Any `T` which implements `ToSql<ST>`
/// - `Option<T>` for any `T` which implements `ToSql<ST>`
///
/// ### [`FromSql`](trait.FromSql.html) impls
///
/// - `Option<T>` for any `T` which implements `FromSql<ST>`
#[derive(Debug, Clone, Copy, Default)]
pub struct Nullable<ST: NotNull>(ST);

#[cfg(feature = "postgres")]
pub use pg::types::sql_types::*;

#[cfg(feature = "mysql")]
pub use mysql::types::*;

/// Indicates that a SQL type exists for a backend.
///
/// # Deriving
///
/// This trait can be automatically derived by `#[derive(SqlType)]`.
/// This derive will also implement [`NotNull`] and [`SingleValue`].
/// When deriving this trait,
/// you need to specify how the type is represented on various backends.
/// You don't need to specify every backend,
/// only the ones supported by your type.
///
/// For PostgreSQL, add `#[postgres(oid = "some_oid", array_oid = "some_oid")]`
/// or `#[postgres(type_name = "pg_type_name")]` if the OID is not stable.
/// For MySQL, specify which variant of [`MysqlType`] should be used
/// by adding `#[mysql_type = "Variant"]`.
/// For SQLite, specify which variant of [`SqliteType`] should be used
/// by adding `#[sqlite_type = "Variant"]`.
///
/// [`NotNull`]: trait.NotNull.html
/// [`SingleValue`]: trait.SingleValue.html
/// [`MysqlType`]: ../mysql/enum.MysqlType.html
/// [`SqliteType`]: ../sqlite/enum.SqliteType.html
///
/// # Example
///
/// ```rust
/// # #[macro_use]
/// # extern crate diesel;
/// #[derive(SqlType)]
/// #[postgres(oid = "23", array_oid = "1007")]
/// #[sqlite_type = "Integer"]
/// #[mysql_type = "Long"]
/// pub struct Integer;
/// # fn main() {}
/// ```
pub trait HasSqlType<ST>: TypeMetadata {
    /// Fetch the metadata for the given type
    ///
    /// This method may use `lookup` to do dynamic runtime lookup. Implementors
    /// of this method should not do dynamic lookup unless absolutely necessary
    fn metadata(lookup: &Self::MetadataLookup) -> Self::TypeMetadata;

    /// Fetch the metadata for a tuple representing an entire row
    ///
    /// The default implementation of this method simply calls `Self::metadata`.
    /// You generally should not need to override this method.
    ///
    /// However, if you are writing an implementation of `HasSqlType` that
    /// simply delegates to an inner type (for example, `Nullable` does this),
    /// then you should ensure that you delegate this method as well.
    fn row_metadata(out: &mut Vec<Self::TypeMetadata>, lookup: &Self::MetadataLookup) {
        out.push(Self::metadata(lookup))
    }
}

/// A marker trait indicating that a SQL type is not null.
///
/// All SQL types must implement this trait.
///
/// # Deriving
///
/// This trait is automatically implemented by `#[derive(SqlType)]`
pub trait NotNull {}

/// Converts a type which may or may not be nullable into its nullable
/// representation.
pub trait IntoNullable {
    /// The nullable representation of this type.
    ///
    /// For all types except `Nullable`, this will be `Nullable<Self>`.
    type Nullable;
}

impl<T: NotNull> IntoNullable for T {
    type Nullable = Nullable<T>;
}

impl<T: NotNull> IntoNullable for Nullable<T> {
    type Nullable = Nullable<T>;
}

/// A marker trait indicating that a SQL type represents a single value, as
/// opposed to a list of values.
///
/// This trait should generally be implemented for all SQL types with the
/// exception of Rust tuples. If a column could have this as its type, this
/// trait should be implemented.
///
/// # Deriving
///
/// This trait is automatically implemented by `#[derive(SqlType)]`
pub trait SingleValue {}

impl<T: NotNull + SingleValue> SingleValue for Nullable<T> {}

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
pub trait FromSql<A, DB: Backend + HasSqlType<A>>: Sized {
    /// See the trait documentation.
    fn from_sql(bytes: Option<&DB::RawValue>) -> Result<Self, Box<Error + Send + Sync>>;
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
pub trait FromSqlRow<A, DB: Backend + HasSqlType<A>>: Sized {
    /// The number of fields that this type will consume. Must be equal to
    /// the number of times you would call `row.take()` in `build_from_row`
    const FIELDS_NEEDED: usize = 1;

    /// See the trait documentation.
    fn build_from_row<T: Row<DB>>(row: &mut T) -> Result<Self, Box<Error + Send + Sync>>;
}

// Reasons we can't write this:
//
// impl<T, ST, DB> FromSqlRow<ST, DB> for T
// where
//     DB: Backend + HasSqlType<ST>,
//     T: FromSql<ST, DB>,
// {
//     fn build_from_row<T: Row<DB>>(row: &mut T) -> Result<Self, Box<Error + Send + Sync>> {
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

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
/// Tiny enum to make the return type of `ToSql` more descriptive
pub enum IsNull {
    /// No data was written, as this type is null
    Yes,
    /// The value is not null
    ///
    /// This does not necessarily mean that any data was written to the buffer.
    /// For example, an empty string has no data to be sent over the wire, but
    /// also is not null.
    No,
}

/// Wraps a buffer to be written by `ToSql` with additional backend specific
/// utilities.
#[derive(Clone, Copy)]
pub struct ToSqlOutput<'a, T, DB>
where
    DB: TypeMetadata,
    DB::MetadataLookup: 'a,
{
    out: T,
    metadata_lookup: &'a DB::MetadataLookup,
}

impl<'a, T, DB: TypeMetadata> ToSqlOutput<'a, T, DB> {
    /// Construct a new `ToSqlOutput`
    pub fn new(out: T, metadata_lookup: &'a DB::MetadataLookup) -> Self {
        ToSqlOutput {
            out,
            metadata_lookup,
        }
    }

    /// Create a new `ToSqlOutput` with the given buffer
    pub fn with_buffer<U>(&self, new_out: U) -> ToSqlOutput<'a, U, DB> {
        ToSqlOutput {
            out: new_out,
            metadata_lookup: self.metadata_lookup,
        }
    }

    /// Return the raw buffer this type is wrapping
    pub fn into_inner(self) -> T {
        self.out
    }

    /// Returns the backend's mechanism for dynamically looking up type
    /// metadata at runtime, if relevant for the given backend.
    pub fn metadata_lookup(&self) -> &'a DB::MetadataLookup {
        self.metadata_lookup
    }
}

#[cfg(test)]
impl<DB: TypeMetadata> ToSqlOutput<'static, Vec<u8>, DB> {
    /// Returns a `ToSqlOutput` suitable for testing `ToSql` implementations.
    /// Unsafe to use for testing types which perform dynamic metadata lookup.
    pub fn test() -> Self {
        use std::mem;
        #[cfg_attr(feature = "clippy", allow(invalid_ref))]
        Self::new(Vec::new(), unsafe { mem::uninitialized() })
    }
}

impl<'a, T: Write, DB: TypeMetadata> Write for ToSqlOutput<'a, T, DB> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.out.write(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.out.flush()
    }

    fn write_all(&mut self, buf: &[u8]) -> io::Result<()> {
        self.out.write_all(buf)
    }

    fn write_fmt(&mut self, fmt: fmt::Arguments) -> io::Result<()> {
        self.out.write_fmt(fmt)
    }
}

impl<'a, T, DB: TypeMetadata> Deref for ToSqlOutput<'a, T, DB> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.out
    }
}

impl<'a, T, DB: TypeMetadata> DerefMut for ToSqlOutput<'a, T, DB> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.out
    }
}

impl<'a, T, U, DB> PartialEq<U> for ToSqlOutput<'a, T, DB>
where
    DB: TypeMetadata,
    T: PartialEq<U>,
{
    fn eq(&self, rhs: &U) -> bool {
        self.out == *rhs
    }
}

impl<'a, T, DB> fmt::Debug for ToSqlOutput<'a, T, DB>
where
    T: fmt::Debug,
    DB: TypeMetadata,
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.out.fmt(f)
    }
}

/// Serializes a single value to be sent to the database.
///
/// The output is sent as a bind parameter, and the data must be written in the
/// expected format for the given backend.
///
/// When possible, implementations of this trait should prefer using an existing
/// implementation, rather than writing to `out` directly. (For example, if you
/// are implementing this for an enum, which is represented as an integer in the
/// database, you should use `i32::to_sql(x, out)` instead of writing to `out`
/// yourself.
///
/// Any types which implement this trait should also `#[derive(AsExpression)]`.
///
/// ### Backend specific details
///
/// - For PostgreSQL, the bytes will be sent using the binary protocol, not text.
/// - For SQLite, all implementations should be written in terms of an existing
///   `ToSql` implementation.
/// - For MySQL, the expected bytes will depend on the return value of
///   `type_metadata` for the given SQL type. See [`MysqlType`] for details.
/// - For third party backends, consult that backend's documentation.
///
/// [`MysqlType`]: ../mysql/enum.MysqlType.html
pub trait ToSql<A, DB: Backend + HasSqlType<A>>: fmt::Debug {
    /// See the trait documentation.
    fn to_sql<W: Write>(
        &self,
        out: &mut ToSqlOutput<W, DB>,
    ) -> Result<IsNull, Box<Error + Send + Sync>>;
}

impl<'a, A, T, DB> ToSql<A, DB> for &'a T
where
    DB: Backend + HasSqlType<A>,
    T: ToSql<A, DB> + ?Sized,
{
    fn to_sql<W: Write>(
        &self,
        out: &mut ToSqlOutput<W, DB>,
    ) -> Result<IsNull, Box<Error + Send + Sync>> {
        (*self).to_sql(out)
    }
}
