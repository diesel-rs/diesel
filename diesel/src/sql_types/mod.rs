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
//! [`FromSql`]: ../deserialize/trait.FromSql.html
//!
//! Any backend specific types are re-exported through this module

mod fold;
pub mod ops;
mod ord;

pub use self::fold::Foldable;
pub use self::ord::SqlOrd;

/// The boolean SQL type.
///
/// On backends without a native boolean type,
/// this is emulated with the smallest supported integer.
///
/// ### [`ToSql`](../serialize/trait.ToSql.html) impls
///
/// - [`bool`][bool]
///
/// ### [`FromSql`](../deserialize/trait.FromSql.html) impls
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
/// not `TinyInt`.
///
/// ### [`ToSql`](../serialize/trait.ToSql.html) impls
///
/// - [`i8`][i8]
///
/// ### [`FromSql`](../deserialize/trait.FromSql.html) impls
///
/// - [`i8`][i8]
///
/// [i8]: https://doc.rust-lang.org/nightly/std/primitive.i8.html
#[derive(Debug, Clone, Copy, Default, QueryId, SqlType)]
#[mysql_type = "Tiny"]
pub struct TinyInt;
#[doc(hidden)]
pub type Tinyint = TinyInt;

/// The small integer SQL type.
///
/// ### [`ToSql`](../serialize/trait.ToSql.html) impls
///
/// - [`i16`][i16]
///
/// ### [`FromSql`](../deserialize/trait.FromSql.html) impls
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
/// ### [`ToSql`](../serialize/trait.ToSql.html) impls
///
/// - [`i32`][i32]
///
/// ### [`FromSql`](../deserialize/trait.FromSql.html) impls
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
/// ### [`ToSql`](../serialize/trait.ToSql.html) impls
///
/// - [`i64`][i64]
///
/// ### [`FromSql`](../deserialize/trait.FromSql.html) impls
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
/// ### [`ToSql`](../serialize/trait.ToSql.html) impls
///
/// - [`f32`][f32]
///
/// ### [`FromSql`](../deserialize/trait.FromSql.html) impls
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
/// ### [`ToSql`](../serialize/trait.ToSql.html) impls
///
/// - [`f64`][f64]
///
/// ### [`FromSql`](../deserialize/trait.FromSql.html) impls
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
/// ### [`ToSql`](../serialize/trait.ToSql.html) impls
///
/// - [`bigdecimal::BigDecimal`] with `feature = ["numeric"]`
///
/// ### [`FromSql`](../deserialize/trait.FromSql.html) impls
///
/// - [`bigdecimal::BigDecimal`] with `feature = ["numeric"]`
///
/// [`bigdecimal::BigDecimal`]: /bigdecimal/struct.BigDecimal.html
#[derive(Debug, Clone, Copy, Default, QueryId, SqlType)]
#[postgres(oid = "1700", array_oid = "1231")]
#[mysql_type = "String"]
#[sqlite_type = "Double"]
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
/// ### [`ToSql`](../serialize/trait.ToSql.html) impls
///
/// - [`String`][String]
/// - [`&str`][str]
///
/// ### [`FromSql`](../deserialize/trait.FromSql.html) impls
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
/// ### [`ToSql`](../serialize/trait.ToSql.html) impls
///
/// - [`Vec<u8>`][Vec]
/// - [`&[u8]`][slice]
///
/// ### [`FromSql`](../deserialize/trait.FromSql.html) impls
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
/// ### [`ToSql`](../serialize/trait.ToSql.html) impls
///
/// - [`chrono::NaiveDate`][NaiveDate] with `feature = "chrono"`
///
/// ### [`FromSql`](../deserialize/trait.FromSql.html) impls
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
/// ### [`ToSql`](../serialize/trait.ToSql.html) impls
///
/// - [`PgInterval`] which can be constructed using [`IntervalDsl`]
///
/// ### [`FromSql`](../deserialize/trait.FromSql.html) impls
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
/// ### [`ToSql`](../serialize/trait.ToSql.html) impls
///
/// - [`chrono::NaiveTime`][NaiveTime] with `feature = "chrono"`
///
/// ### [`FromSql`](../deserialize/trait.FromSql.html) impls
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
/// ### [`ToSql`](../serialize/trait.ToSql.html) impls
///
/// - [`std::time::SystemTime`][SystemTime] (PG only)
/// - [`chrono::NaiveDateTime`][NaiveDateTime] with `feature = "chrono"`
/// - [`time::Timespec`][Timespec] with `feature = "deprecated-time"` (PG only)
///
/// ### [`FromSql`](../deserialize/trait.FromSql.html) impls
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
/// ### [`ToSql`](../serialize/trait.ToSql.html) impls
///
/// - Any `T` which implements `ToSql<ST>`
/// - `Option<T>` for any `T` which implements `ToSql<ST>`
///
/// ### [`FromSql`](../deserialize/trait.FromSql.html) impls
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

    #[doc(hidden)]
    #[cfg(feature = "mysql")]
    fn is_signed() -> IsSigned {
        IsSigned::Signed
    }

    #[doc(hidden)]
    #[cfg(feature = "mysql")]
    fn mysql_metadata(lookup: &Self::MetadataLookup) -> (Self::TypeMetadata, IsSigned) {
        (Self::metadata(lookup), Self::is_signed())
    }

    #[doc(hidden)]
    #[cfg(feature = "mysql")]
    fn mysql_row_metadata(
        out: &mut Vec<(Self::TypeMetadata, IsSigned)>,
        lookup: &Self::MetadataLookup,
    ) {
        out.push(Self::mysql_metadata(lookup))
    }
}

#[doc(hidden)]
#[cfg(feature = "mysql")]
#[derive(Debug, Clone, Copy)]
pub enum IsSigned {
    Signed,
    Unsigned,
}

/// Information about how a backend stores metadata about given SQL types
pub trait TypeMetadata {
    /// The actual type used to represent metadata.
    ///
    /// On PostgreSQL, this is the type's OID.
    /// On MySQL and SQLite, this is an enum representing all storage classes
    /// they support.
    type TypeMetadata;
    /// The type used for runtime lookup of metadata.
    ///
    /// For most backends, which don't support user defined types, this will
    /// be `()`.
    type MetadataLookup;
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
