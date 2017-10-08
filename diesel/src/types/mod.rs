//! Types which represent a native SQL data type, and the conversions between
//! them and Rust primitives. The structs in this module are *only* used as
//! markers to represent a SQL type, and shouldn't be used in your structs. See
//! the documentation for each type to see the Rust types that can be used with
//! a corresponding SQL type. Additional types can be added by other crates.
//!
//! To see which Rust types can be used with a given SQL type, see the
//! "Implementors" section of the [`ToSql`][ToSql] and [`FromSql`][FromSql]
//! traits, or see the documentation for that SQL type.
//!
//! [ToSql]: /diesel/types/trait.ToSql.html
//! [FromSql]: /diesel/types/trait.FromSql.html
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

/// The boolean SQL type. On SQLite this is emulated with an integer.
///
/// ### [`ToSql`](/diesel/types/trait.ToSql.html) impls
///
/// - [`bool`][bool]
///
/// ### [`FromSql`](/diesel/types/trait.FromSql.html) impls
///
/// - [`bool`][bool]
///
/// [bool]: https://doc.rust-lang.org/nightly/std/primitive.bool.html
#[derive(Debug, Clone, Copy, Default)]
pub struct Bool;

/// The tinyint SQL type. This is only available on MySQL.
///
/// ### [`ToSql`](/diesel/types/trait.ToSql.html) impls
///
/// - [`i8`][i8]
///
/// ### [`FromSql`](/diesel/types/trait.FromSql.html) impls
///
/// - [`i8`][i8]
///
/// [i8]: https://doc.rust-lang.org/nightly/std/primitive.i8.html
#[derive(Debug, Clone, Copy, Default)]
pub struct Tinyint;

/// The small integer SQL type.
///
/// ### [`ToSql`](/diesel/types/trait.ToSql.html) impls
///
/// - [`i16`][i16]
///
/// ### [`FromSql`](/diesel/types/trait.FromSql.html) impls
///
/// - [`i16`][i16]
///
/// [i16]: https://doc.rust-lang.org/nightly/std/primitive.i16.html
#[derive(Debug, Clone, Copy, Default)]
pub struct SmallInt;
#[doc(hidden)]
pub type Int2 = SmallInt;
#[doc(hidden)]
pub type Smallint = SmallInt;

/// The integer SQL type.
///
/// ### [`ToSql`](/diesel/types/trait.ToSql.html) impls
///
/// - [`i32`][i32]
///
/// ### [`FromSql`](/diesel/types/trait.FromSql.html) impls
///
/// - [`i32`][i32]
///
/// [i32]: https://doc.rust-lang.org/nightly/std/primitive.i32.html
#[derive(Debug, Clone, Copy, Default)]
pub struct Integer;
#[doc(hidden)]
pub type Int4 = Integer;

/// The big integer SQL type.
///
/// ### [`ToSql`](/diesel/types/trait.ToSql.html) impls
///
/// - [`i64`][i64]
///
/// ### [`FromSql`](/diesel/types/trait.FromSql.html) impls
///
/// - [`i64`][i64]
///
/// [i64]: https://doc.rust-lang.org/nightly/std/primitive.i64.html
#[derive(Debug, Clone, Copy, Default)]
pub struct BigInt;
#[doc(hidden)]
pub type Int8 = BigInt;
#[doc(hidden)]
pub type Bigint = BigInt;

/// The float SQL type.
///
/// ### [`ToSql`](/diesel/types/trait.ToSql.html) impls
///
/// - [`f32`][f32]
///
/// ### [`FromSql`](/diesel/types/trait.FromSql.html) impls
///
/// - [`f32`][f32]
///
/// [f32]: https://doc.rust-lang.org/nightly/std/primitive.f32.html
#[derive(Debug, Clone, Copy, Default)]
pub struct Float;
#[doc(hidden)]
pub type Float4 = Float;

/// The double precision float SQL type.
///
/// ### [`ToSql`](/diesel/types/trait.ToSql.html) impls
///
/// - [`f64`][f64]
///
/// ### [`FromSql`](/diesel/types/trait.FromSql.html) impls
///
/// - [`f64`][f64]
///
/// [f64]: https://doc.rust-lang.org/nightly/std/primitive.f64.html
#[derive(Debug, Clone, Copy, Default)]
pub struct Double;
#[doc(hidden)]
pub type Float8 = Double;

/// The numeric SQL type.
///
/// ### [`ToSql`](/diesel/types/trait.ToSql.html) impls
///
/// - [`bigdecimal::BigDecimal`][bigdecimal] (currently PostgreSQL and MySQL only, requires the `numeric`
/// feature, which depends on the
/// [`bigdecimal`][bigdecimal] crate)
///
/// ### [`FromSql`](/diesel/types/trait.FromSql.html) impls
///
/// - [`bigdecimal::BigDecimal`][BigDecimal] (currently PostgreSQL and MySQL only, requires the `numeric`
/// feature, which depends on the
/// [`bigdecimal`][bigdecimal] crate)
///
/// On SQLite, [`Double`](struct.Double.html) should be used instead.
///
/// [BigDecimal]: /bigdecimal/struct.BigDecimal.html
/// [bigdecimal]: /bigdecimal/index.html
#[derive(Debug, Clone, Copy, Default)]
pub struct Numeric;
pub type Decimal = Numeric;

#[cfg(not(feature = "postgres"))]
impl NotNull for Numeric {}

#[cfg(not(feature = "postgres"))]
impl SingleValue for Numeric {}

/// The text SQL type.
///
/// On all backends strings must be valid UTF-8.
/// On PostgreSQL strings must not include nul bytes.
///
/// On MySQL, it is also aliased by `Tinytext`, `Mediumtext`, `Longtext`, `Char` and `Varchar`.
///
/// ### [`ToSql`](/diesel/types/trait.ToSql.html) impls
///
/// - [`String`][String]
/// - [`&str`][str]
///
/// ### [`FromSql`](/diesel/types/trait.FromSql.html) impls
///
/// - [`String`][String]
///
/// [String]: https://doc.rust-lang.org/nightly/std/string/struct.String.html
/// [str]: https://doc.rust-lang.org/nightly/std/primitive.str.html
#[derive(Debug, Clone, Copy, Default)]
pub struct Text;
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
/// On MySQL, it is also aliased by `Tinyblob`, `Blob`, `Mediumblob`, `Longblob`, `Bit` and `Varbinary`.
///
/// ### [`ToSql`](/diesel/types/trait.ToSql.html) impls
///
/// - [`Vec<u8>`][Vec]
/// - [`&[u8]`][slice]
///
/// ### [`FromSql`](/diesel/types/trait.FromSql.html) impls
///
/// - [`Vec<u8>`][Vec]
///
/// [Vec]: https://doc.rust-lang.org/nightly/std/vec/struct.Vec.html
/// [slice]: https://doc.rust-lang.org/nightly/std/primitive.slice.html
#[derive(Debug, Clone, Copy, Default)]
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
/// This type is currently only implemented for PostgreSQL and SQLite.
///
/// ### [`ToSql`](/diesel/types/trait.ToSql.html) impls
///
/// - [`chrono::NaiveDate`][NaiveDate] with `feature = "chrono"`
///
/// ### [`FromSql`](/diesel/types/trait.FromSql.html) impls
///
/// - [`chrono::NaiveDate`][NaiveDate] with `feature = "chrono"`
///
/// [NaiveDate]: /chrono/naive/date/struct.NaiveDate.html
#[derive(Debug, Clone, Copy, Default)]
pub struct Date;

/// The interval SQL type.
///
/// This type is currently only implemented for PostgreSQL.
///
/// ### [`ToSql`](/diesel/types/trait.ToSql.html) impls
///
/// - [`PgInterval`][PgInterval] which can be constructed using the [interval
///   DSLs][interval dsls]
///
/// ### [`FromSql`](/diesel/types/trait.FromSql.html) impls
///
/// - [`PgInterval`][PgInterval] which can be constructed using the [interval
///   DSLs][interval dsls]
///
/// [PgInterval]: /diesel/pg/data_types/struct.PgInterval.html
/// [interval dsls]: /diesel/pg/expression/extensions/index.html
#[derive(Debug, Clone, Copy, Default)]
pub struct Interval;

#[cfg(not(feature = "postgres"))]
impl NotNull for Interval {} // FIXME: Interval should not be in this file

/// The time SQL type.
///
/// This type is currently only implemented for PostgreSQL and SQLite.
///
/// ### [`ToSql`](/diesel/types/trait.ToSql.html) impls
///
/// - [`chrono::NaiveTime`][NaiveTime] with `feature = "chrono"`
///
/// ### [`FromSql`](/diesel/types/trait.FromSql.html) impls
///
/// - [`chrono::NaiveTime`][NaiveTime] with `feature = "chrono"`
///
/// [NaiveTime]: /chrono/naive/time/struct.NaiveTime.html
#[derive(Debug, Clone, Copy, Default)]
pub struct Time;

/// The timestamp/datetime SQL type.
///
/// This type is currently only implemented for PostgreSQL and SQLite.
///
/// ### [`ToSql`](/diesel/types/trait.ToSql.html) impls
///
/// - [`std::time::SystemTime`][SystemTime] (PG only)
/// - [`chrono::NaiveDateTime`][NaiveDateTime] with `feature = "chrono"`
/// - [`time::Timespec`][Timespec] with `feature = "deprecated-time"` (PG only)
///
/// ### [`FromSql`](/diesel/types/trait.FromSql.html) impls
///
/// - [`std::time::SystemTime`][SystemTime] (PG only)
/// - [`chrono::NaiveDateTime`][NaiveDateTime] with `feature = "chrono"`
/// - [`time::Timespec`][Timespec] with `feature = "deprecated-time"` (PG only)
///
/// [SystemTime]: https://doc.rust-lang.org/nightly/std/time/struct.SystemTime.html
/// [NaiveDateTime]: /chrono/naive/datetime/struct.NaiveDateTime.html
/// [Timespec]: /time/struct.Timespec.html
#[derive(Debug, Clone, Copy, Default)]
pub struct Timestamp;

/// The nullable SQL type. This wraps another SQL type to indicate that it can
/// be null. By default all values are assumed to be `NOT NULL`.
///
/// ### [`ToSql`](/diesel/types/trait.ToSql.html) impls
///
/// - Any `T` which implements `ToSql<ST>`
/// - `Option<T>` for any `T` which implements `ToSql<ST>`
///
/// ### [`FromSql`](/diesel/types/trait.FromSql.html) impls
///
/// - `Option<T>` for any `T` which implements `FromSql<ST>`
#[derive(Debug, Clone, Copy, Default)]
pub struct Nullable<ST: NotNull>(ST);

#[cfg(feature = "postgres")]
pub use pg::types::sql_types::*;

#[cfg(feature = "mysql")]
pub use mysql::types::*;

pub trait HasSqlType<ST>: TypeMetadata {
    fn metadata(lookup: &Self::MetadataLookup) -> Self::TypeMetadata;

    fn row_metadata(out: &mut Vec<Self::TypeMetadata>, lookup: &Self::MetadataLookup) {
        out.push(Self::metadata(lookup))
    }
}

pub trait NotNull {}

pub trait IntoNullable {
    type Nullable;
}

impl<T: NotNull> IntoNullable for T {
    type Nullable = Nullable<T>;
}

impl<T: NotNull> IntoNullable for Nullable<T> {
    type Nullable = Nullable<T>;
}

pub trait SingleValue {}

impl<T: NotNull + SingleValue> SingleValue for Nullable<T> {}

/// How to deserialize a single field of a given type. The input will always be
/// the binary representation, not the text.
pub trait FromSql<A, DB: Backend + HasSqlType<A>>: Sized {
    fn from_sql(bytes: Option<&DB::RawValue>) -> Result<Self, Box<Error + Send + Sync>>;
}

/// How to deserialize multiple fields, with a known type. This type is
/// implemented for tuples of various sizes.
pub trait FromSqlRow<A, DB: Backend + HasSqlType<A>>: Sized {
    /// The number of fields that this type will consume. Should be equal to
    /// the number of times you would call `row.take()` in `build_from_row`
    const FIELDS_NEEDED: usize = 1;

    fn build_from_row<T: Row<DB>>(row: &mut T) -> Result<Self, Box<Error + Send + Sync>>;
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
/// Tiny enum to make the return type of `ToSql` more descriptive
pub enum IsNull {
    Yes,
    No,
}

#[derive(Clone, Copy)]
#[doc(hidden)]
pub struct ToSqlOutput<'a, T, DB>
where
    DB: TypeMetadata,
    DB::MetadataLookup: 'a,
{
    out: T,
    metadata_lookup: &'a DB::MetadataLookup,
}

impl<'a, T, DB: TypeMetadata> ToSqlOutput<'a, T, DB> {
    pub fn new(out: T, metadata_lookup: &'a DB::MetadataLookup) -> Self {
        ToSqlOutput {
            out,
            metadata_lookup,
        }
    }

    pub fn with_buffer<U>(&self, new_out: U) -> ToSqlOutput<'a, U, DB> {
        ToSqlOutput {
            out: new_out,
            metadata_lookup: self.metadata_lookup,
        }
    }

    pub fn into_inner(self) -> T {
        self.out
    }

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

/// Serializes a single value to be sent to the database. The output will be
/// included as a bind parameter, and is expected to be the binary format, not
/// text.
pub trait ToSql<A, DB: Backend + HasSqlType<A>>: fmt::Debug {
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
