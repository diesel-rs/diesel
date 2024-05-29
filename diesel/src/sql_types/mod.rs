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
//! [`FromSql`]: super::deserialize::FromSql
//!
//! Any backend specific types are re-exported through this module

mod fold;
pub mod ops;
mod ord;

pub use self::fold::Foldable;
pub use self::ord::SqlOrd;

use crate::expression::TypedExpressionType;
use crate::query_builder::QueryId;

/// The boolean SQL type.
///
/// On backends without a native boolean type,
/// this is emulated with the smallest supported integer.
///
/// ### [`ToSql`](crate::serialize::ToSql) impls
///
/// - [`bool`][bool]
///
/// ### [`FromSql`](crate::deserialize::FromSql) impls
///
/// - [`bool`][bool]
///
/// [bool]: https://doc.rust-lang.org/nightly/std/primitive.bool.html
#[derive(Debug, Clone, Copy, Default, QueryId, SqlType)]
#[diesel(postgres_type(oid = 16, array_oid = 1000))]
#[diesel(sqlite_type(name = "Integer"))]
#[diesel(mysql_type(name = "Tiny"))]
pub struct Bool;

/// The tiny integer SQL type.
///
/// This is only available on MySQL.
/// Keep in mind that `diesel print-schema` will see `TINYINT(1)` as `Bool`,
/// not `TinyInt`.
///
/// ### [`ToSql`](crate::serialize::ToSql) impls
///
/// - [`i8`][i8]
///
/// ### [`FromSql`](crate::deserialize::FromSql) impls
///
/// - [`i8`][i8]
///
/// [i8]: https://doc.rust-lang.org/nightly/std/primitive.i8.html
#[derive(Debug, Clone, Copy, Default, QueryId, SqlType)]
#[diesel(mysql_type(name = "Tiny"))]
pub struct TinyInt;
#[doc(hidden)]
pub type Tinyint = TinyInt;

/// The small integer SQL type.
///
/// ### [`ToSql`](crate::serialize::ToSql) impls
///
/// - [`i16`][i16]
///
/// ### [`FromSql`](crate::deserialize::FromSql) impls
///
/// - [`i16`][i16]
///
/// [i16]: https://doc.rust-lang.org/nightly/std/primitive.i16.html
#[derive(Debug, Clone, Copy, Default, QueryId, SqlType)]
#[diesel(postgres_type(oid = 21, array_oid = 1005))]
#[diesel(sqlite_type(name = "SmallInt"))]
#[diesel(mysql_type(name = "Short"))]
pub struct SmallInt;
#[doc(hidden)]
pub type Int2 = SmallInt;
#[doc(hidden)]
pub type Smallint = SmallInt;

/// The integer SQL type.
///
/// ### [`ToSql`](crate::serialize::ToSql) impls
///
/// - [`i32`][i32]
///
/// ### [`FromSql`](crate::deserialize::FromSql) impls
///
/// - [`i32`][i32]
///
/// [i32]: https://doc.rust-lang.org/nightly/std/primitive.i32.html
#[derive(Debug, Clone, Copy, Default, QueryId, SqlType)]
#[diesel(postgres_type(oid = 23, array_oid = 1007))]
#[diesel(sqlite_type(name = "Integer"))]
#[diesel(mysql_type(name = "Long"))]
pub struct Integer;
#[doc(hidden)]
pub type Int4 = Integer;

/// The big integer SQL type.
///
/// ### [`ToSql`](crate::serialize::ToSql) impls
///
/// - [`i64`][i64]
///
/// ### [`FromSql`](crate::deserialize::FromSql) impls
///
/// - [`i64`][i64]
///
/// [i64]: https://doc.rust-lang.org/nightly/std/primitive.i64.html
#[derive(Debug, Clone, Copy, Default, QueryId, SqlType)]
#[diesel(postgres_type(oid = 20, array_oid = 1016))]
#[diesel(sqlite_type(name = "Long"))]
#[diesel(mysql_type(name = "LongLong"))]
pub struct BigInt;
#[doc(hidden)]
pub type Int8 = BigInt;
#[doc(hidden)]
pub type Bigint = BigInt;

/// The float SQL type.
///
/// ### [`ToSql`](crate::serialize::ToSql) impls
///
/// - [`f32`][f32]
///
/// ### [`FromSql`](crate::deserialize::FromSql) impls
///
/// - [`f32`][f32]
///
/// [f32]: https://doc.rust-lang.org/nightly/std/primitive.f32.html
#[derive(Debug, Clone, Copy, Default, QueryId, SqlType)]
#[diesel(postgres_type(oid = 700, array_oid = 1021))]
#[diesel(sqlite_type(name = "Float"))]
#[diesel(mysql_type(name = "Float"))]
pub struct Float;
#[doc(hidden)]
pub type Float4 = Float;

/// The double precision float SQL type.
///
/// ### [`ToSql`](crate::serialize::ToSql) impls
///
/// - [`f64`][f64]
///
/// ### [`FromSql`](crate::deserialize::FromSql) impls
///
/// - [`f64`][f64]
///
/// [f64]: https://doc.rust-lang.org/nightly/std/primitive.f64.html
#[derive(Debug, Clone, Copy, Default, QueryId, SqlType)]
#[diesel(postgres_type(oid = 701, array_oid = 1022))]
#[diesel(sqlite_type(name = "Double"))]
#[diesel(mysql_type(name = "Double"))]
pub struct Double;
#[doc(hidden)]
pub type Float8 = Double;

/// The arbitrary precision numeric SQL type.
///
/// This type is only supported on PostgreSQL and MySQL.
/// On SQLite, [`Double`] should be used instead.
///
/// ### [`ToSql`](crate::serialize::ToSql) impls
///
/// - [`bigdecimal::BigDecimal`] with `feature = ["numeric"]`
///
/// ### [`FromSql`](crate::deserialize::FromSql) impls
///
/// - [`bigdecimal::BigDecimal`] with `feature = ["numeric"]`
///
/// [`bigdecimal::BigDecimal`]: /bigdecimal/struct.BigDecimal.html
#[derive(Debug, Clone, Copy, Default, QueryId, SqlType)]
#[diesel(postgres_type(oid = 1700, array_oid = 1231))]
#[diesel(mysql_type(name = "Numeric"))]
#[diesel(sqlite_type(name = "Double"))]
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
/// ### [`ToSql`](crate::serialize::ToSql) impls
///
/// - [`String`]
/// - [`&str`][str]
///
/// ### [`FromSql`](crate::deserialize::FromSql) impls
///
/// - [`String`]
///
/// [str]: https://doc.rust-lang.org/nightly/std/primitive.str.html
#[derive(Debug, Clone, Copy, Default, QueryId, SqlType)]
#[diesel(postgres_type(oid = 25, array_oid = 1009))]
#[diesel(sqlite_type(name = "Text"))]
#[diesel(mysql_type(name = "String"))]
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
/// ### [`ToSql`](crate::serialize::ToSql) impls
///
/// - [`Vec<u8>`][Vec]
/// - [`&[u8]`][slice]
///
/// ### [`FromSql`](crate::deserialize::FromSql) impls
///
/// - [`Vec<u8>`][Vec]
///
/// [Vec]: std::vec::Vec
/// [slice]: https://doc.rust-lang.org/nightly/std/primitive.slice.html
#[derive(Debug, Clone, Copy, Default, QueryId, SqlType)]
#[diesel(postgres_type(oid = 17, array_oid = 1001))]
#[diesel(sqlite_type(name = "Binary"))]
#[diesel(mysql_type(name = "Blob"))]
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
/// ### [`ToSql`](crate::serialize::ToSql) impls
///
/// - [`chrono::NaiveDate`][NaiveDate] with `feature = "chrono"`
/// - [`time::Date`][Date] with `feature = "time"`
///
/// ### [`FromSql`](crate::deserialize::FromSql) impls
///
/// - [`chrono::NaiveDate`][NaiveDate] with `feature = "chrono"`
/// - [`time::Date`][Date] with `feature = "time"`
///
/// [NaiveDate]: https://docs.rs/chrono/*/chrono/naive/struct.NaiveDate.html
/// [Date]: https://docs.rs/time/0.3.9/time/struct.Date.html
#[derive(Debug, Clone, Copy, Default, QueryId, SqlType)]
#[diesel(postgres_type(oid = 1082, array_oid = 1182))]
#[diesel(sqlite_type(name = "Text"))]
#[diesel(mysql_type(name = "Date"))]
pub struct Date;

/// The interval SQL type.
///
/// This type is currently only implemented for PostgreSQL.
///
/// ### [`ToSql`](crate::serialize::ToSql) impls
///
/// - [`PgInterval`] which can be constructed using [`IntervalDsl`]
/// - [`chrono::Duration`][Duration] with `feature = "chrono"`
///
/// ### [`FromSql`](crate::deserialize::FromSql) impls
///
/// - [`PgInterval`] which can be constructed using [`IntervalDsl`]
/// - [`chrono::Duration`][Duration] with `feature = "chrono"`
///     (There might be some information loss due to special behavior for literal `month` (or longer) intervals;
///      Please read official documentation of [PostgreSQL Interval].)
///
/// [`PgInterval`]: ../pg/data_types/struct.PgInterval.html
/// [`IntervalDsl`]: ../pg/expression/extensions/trait.IntervalDsl.html
/// [Duration]: https://docs.rs/chrono/*/chrono/type.Duration.html
/// [PostgreSQL Interval]: https://www.postgresql.org/docs/current/datatype-datetime.html#DATATYPE-INTERVAL-INPUT
#[derive(Debug, Clone, Copy, Default, QueryId, SqlType)]
#[diesel(postgres_type(oid = 1186, array_oid = 1187))]
pub struct Interval;

/// The time SQL type.
///
/// ### [`ToSql`](crate::serialize::ToSql) impls
///
/// - [`chrono::NaiveTime`][NaiveTime] with `feature = "chrono"`
/// - [`time::Time`][Time] with `feature = "time"`
///
/// ### [`FromSql`](crate::deserialize::FromSql) impls
///
/// - [`chrono::NaiveTime`][NaiveTime] with `feature = "chrono"`
/// - [`time::Time`][Time] with `feature = "time"`
///
/// [NaiveTime]: /chrono/naive/time/struct.NaiveTime.html
/// [Time]: /time/struct.Time.html
#[derive(Debug, Clone, Copy, Default, QueryId, SqlType)]
#[diesel(postgres_type(oid = 1083, array_oid = 1183))]
#[diesel(sqlite_type(name = "Text"))]
#[diesel(mysql_type(name = "Time"))]
pub struct Time;

/// The timestamp SQL type.
///
/// ### [`ToSql`](crate::serialize::ToSql) impls
///
/// - [`std::time::SystemTime`][SystemTime] (PG only)
/// - [`chrono::NaiveDateTime`][NaiveDateTime] with `feature = "chrono"`
/// - [`time::PrimitiveDateTime`] with `feature = "time"`
/// - [`time::OffsetDateTime`] with `feature = "time"` (MySQL only)
///
/// ### [`FromSql`](crate::deserialize::FromSql) impls
///
/// - [`std::time::SystemTime`][SystemTime] (PG only)
/// - [`chrono::NaiveDateTime`][NaiveDateTime] with `feature = "chrono"`
/// - [`time::PrimitiveDateTime`] with `feature = "time"`
/// - [`time::OffsetDateTime`] with `feature = "time"` (MySQL only)
///
/// [SystemTime]: std::time::SystemTime
#[cfg_attr(
    feature = "chrono",
    doc = " [NaiveDateTime]: chrono::naive::NaiveDateTime"
)]
#[cfg_attr(
    not(feature = "chrono"),
    doc = " [NaiveDateTime]: https://docs.rs/chrono/*/chrono/naive/struct.NaiveDateTime.html"
)]
#[cfg_attr(
    feature = "time",
    doc = " [`time::PrimitiveDateTime`]: time::PrimitiveDateTime"
)]
#[cfg_attr(
    not(feature = "time"),
    doc = " [`time::PrimitiveDateTime`]: https://docs.rs/time/0.3.9/time/struct.PrimitiveDateTime.html"
)]
#[cfg_attr(
    feature = "time",
    doc = " [`time::OffsetDateTime`]: time::OffsetDateTime"
)]
#[cfg_attr(
    not(feature = "time"),
    doc = " [`time::OffsetDateTime`]: https://docs.rs/time/0.3.9/time/struct.OffsetDateTime.html"
)]
/// [Timespec]: /time/struct.Timespec.html
#[derive(Debug, Clone, Copy, Default, QueryId, SqlType)]
#[diesel(postgres_type(oid = 1114, array_oid = 1115))]
#[diesel(sqlite_type(name = "Text"))]
#[diesel(mysql_type(name = "Timestamp"))]
pub struct Timestamp;

/// The JSON SQL type.  This type can only be used with `feature =
/// "serde_json"`
///
/// For postgresql you should normally prefer [`Jsonb`](struct.Jsonb.html) instead,
/// for the reasons discussed there.
///
/// ### [`ToSql`] impls
///
/// - [`serde_json::Value`]
///
/// ### [`FromSql`] impls
///
/// - [`serde_json::Value`]
///
/// [`ToSql`]: /serialize/trait.ToSql.html
/// [`FromSql`]: /deserialize/trait.FromSql.html
/// [`serde_json::Value`]: /../serde_json/value/enum.Value.html
#[derive(Debug, Clone, Copy, Default, QueryId, SqlType)]
#[diesel(postgres_type(oid = 114, array_oid = 199))]
#[diesel(mysql_type(name = "String"))]
pub struct Json;

/// The nullable SQL type.
///
/// This wraps another SQL type to indicate that it can be null.
/// By default all values are assumed to be `NOT NULL`.
///
/// ### [`ToSql`](crate::serialize::ToSql) impls
///
/// - Any `T` which implements `ToSql<ST>`
/// - `Option<T>` for any `T` which implements `ToSql<ST>`
///
/// ### [`FromSql`](crate::deserialize::FromSql) impls
///
/// - `Option<T>` for any `T` which implements `FromSql<ST>`
#[derive(Debug, Clone, Copy, Default)]
pub struct Nullable<ST>(ST);

impl<ST> SqlType for Nullable<ST>
where
    ST: SqlType,
{
    type IsNull = is_nullable::IsNullable;
}

#[doc(inline)]
#[cfg(feature = "postgres_backend")]
pub use crate::pg::sql_types::*;

#[doc(inline)]
#[cfg(feature = "mysql_backend")]
pub use crate::mysql::sql_types::{Datetime, Unsigned};

#[doc(inline)]
#[cfg(feature = "sqlite")]
pub use crate::sqlite::sql_types::Timestamptz as TimestamptzSqlite;

/// Indicates that a SQL type exists for a backend.
///
/// This trait can be derived using the [`SqlType` derive](derive@SqlType)
///
/// # Example
///
/// ```rust
/// #[derive(diesel::sql_types::SqlType)]
/// #[diesel(postgres_type(oid = 23, array_oid = 1007))]
/// #[diesel(sqlite_type(name = "Integer"))]
/// #[diesel(mysql_type(name = "Long"))]
/// pub struct Integer;
/// ```
pub trait HasSqlType<ST>: TypeMetadata {
    /// Fetch the metadata for the given type
    ///
    /// This method may use `lookup` to do dynamic runtime lookup. Implementors
    /// of this method should not do dynamic lookup unless absolutely necessary
    fn metadata(lookup: &mut Self::MetadataLookup) -> Self::TypeMetadata;
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
    type MetadataLookup: ?Sized;
}

/// Converts a type which may or may not be nullable into its nullable
/// representation.
pub trait IntoNullable {
    /// The nullable representation of this type.
    ///
    /// For all types except `Nullable`, this will be `Nullable<Self>`.
    type Nullable;
}

impl<T> IntoNullable for T
where
    T: SqlType<IsNull = is_nullable::NotNull> + SingleValue,
{
    type Nullable = Nullable<T>;
}

impl<T> IntoNullable for Nullable<T>
where
    T: SqlType,
{
    type Nullable = Self;
}

/// Converts a type which may or may not be nullable into its not nullable
/// representation.
pub trait IntoNotNullable {
    /// The not nullable representation of this type.
    ///
    /// For `Nullable<T>`, this will be `T` otherwise the type itself
    type NotNullable;
}

impl<T> IntoNotNullable for T
where
    T: SqlType<IsNull = is_nullable::NotNull>,
{
    type NotNullable = T;
}

impl<T> IntoNotNullable for Nullable<T>
where
    T: SqlType,
{
    type NotNullable = T;
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
/// This trait is automatically implemented by [`#[derive(SqlType)]`](derive@SqlType)
///
pub trait SingleValue: SqlType {}

impl<T: SqlType + SingleValue> SingleValue for Nullable<T> {}

#[doc(inline)]
pub use diesel_derives::DieselNumericOps;
#[doc(inline)]
pub use diesel_derives::SqlType;

/// A marker trait for SQL types
///
/// # Deriving
///
/// This trait is automatically implemented by [`#[derive(SqlType)]`](derive@SqlType)
/// which sets `IsNull` to [`is_nullable::NotNull`]
///
pub trait SqlType: 'static {
    /// Is this type nullable?
    ///
    /// This type should always be one of the structs in the ['is_nullable`]
    /// module. See the documentation of those structs for more details.
    ///
    /// ['is_nullable`]: is_nullable
    type IsNull: OneIsNullable<is_nullable::IsNullable> + OneIsNullable<is_nullable::NotNull>;
}

/// Is one value of `IsNull` nullable?
///
/// You should never implement this trait.
pub trait OneIsNullable<Other> {
    /// See the trait documentation
    type Out: OneIsNullable<is_nullable::IsNullable> + OneIsNullable<is_nullable::NotNull>;
}

/// Are both values of `IsNull` are nullable?
pub trait AllAreNullable<Other> {
    /// See the trait documentation
    type Out: AllAreNullable<is_nullable::NotNull> + AllAreNullable<is_nullable::IsNullable>;
}

/// A type level constructor for maybe nullable types
///
/// Constructs either `Nullable<O>` (for `Self` == `is_nullable::IsNullable`)
/// or `O` (for `Self` == `is_nullable::NotNull`)
pub trait MaybeNullableType<O> {
    /// See the trait documentation
    type Out: SqlType + TypedExpressionType;
}

/// Possible values for `SqlType::IsNullable`
pub mod is_nullable {
    use super::*;

    /// No, this type cannot be null as it is marked as `NOT NULL` at database level
    ///
    /// This should be chosen for basically all manual impls of `SqlType`
    /// beside implementing your own `Nullable<>` wrapper type
    #[derive(Debug, Clone, Copy)]
    pub struct NotNull;

    /// Yes, this type can be null
    ///
    /// The only diesel provided `SqlType` that uses this value is [`Nullable<T>`]
    ///
    /// [`Nullable<T>`]: Nullable
    #[derive(Debug, Clone, Copy)]
    pub struct IsNullable;

    impl OneIsNullable<NotNull> for NotNull {
        type Out = NotNull;
    }

    impl OneIsNullable<IsNullable> for NotNull {
        type Out = IsNullable;
    }

    impl OneIsNullable<NotNull> for IsNullable {
        type Out = IsNullable;
    }

    impl OneIsNullable<IsNullable> for IsNullable {
        type Out = IsNullable;
    }

    impl AllAreNullable<NotNull> for NotNull {
        type Out = NotNull;
    }

    impl AllAreNullable<IsNullable> for NotNull {
        type Out = NotNull;
    }

    impl AllAreNullable<NotNull> for IsNullable {
        type Out = NotNull;
    }

    impl AllAreNullable<IsNullable> for IsNullable {
        type Out = IsNullable;
    }

    impl<O> MaybeNullableType<O> for NotNull
    where
        O: SqlType + TypedExpressionType,
    {
        type Out = O;
    }

    impl<O> MaybeNullableType<O> for IsNullable
    where
        O: SqlType,
        Nullable<O>: TypedExpressionType,
    {
        type Out = Nullable<O>;
    }

    /// Represents the output type of [`MaybeNullableType`]
    pub type MaybeNullable<N, T> = <N as MaybeNullableType<T>>::Out;

    /// Represents the output type of [`OneIsNullable`]
    /// for two given SQL types
    pub type IsOneNullable<S1, S2> =
        <IsSqlTypeNullable<S1> as OneIsNullable<IsSqlTypeNullable<S2>>>::Out;

    /// Represents the output type of [`AllAreNullable`]
    /// for two given SQL types
    pub type AreAllNullable<S1, S2> =
        <IsSqlTypeNullable<S1> as AllAreNullable<IsSqlTypeNullable<S2>>>::Out;

    /// Represents if the SQL type is nullable or not
    pub type IsSqlTypeNullable<T> = <T as SqlType>::IsNull;
}

/// A marker trait for accepting expressions of the type `Bool` and
/// `Nullable<Bool>` in the same place
#[diagnostic::on_unimplemented(
    message = "`{Self}` is neither `diesel::sql_types::Bool` nor `diesel::sql_types::Nullable<Bool>`",
    note = "try to provide an expression that produces one of the expected sql types"
)]
pub trait BoolOrNullableBool {}

impl BoolOrNullableBool for Bool {}
impl BoolOrNullableBool for Nullable<Bool> {}

#[doc(inline)]
pub use crate::expression::expression_types::Untyped;
