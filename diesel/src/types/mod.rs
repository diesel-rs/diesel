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

/// Marker trait for types which can be compared for ordering.
pub use self::ord::SqlOrd;

/// Marker trait for types which can be folded for a sum.
pub use self::fold::Foldable;

use backend::{Backend, TypeMetadata};
use row::Row;
use std::error::Error;
use std::io::Write;

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
#[derive(Debug, Clone, Copy, Default)] pub struct Bool;

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
#[derive(Debug, Clone, Copy, Default)] pub struct SmallInt;
#[doc(hidden)] pub type Int2 = SmallInt;

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
#[derive(Debug, Clone, Copy, Default)] pub struct Integer;
#[doc(hidden)] pub type Int4 = Integer;

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
#[derive(Debug, Clone, Copy, Default)] pub struct BigInt;
#[doc(hidden)] pub type Int8 = BigInt;

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
#[derive(Debug, Clone, Copy, Default)] pub struct Float;
#[doc(hidden)] pub type Float4 = Float;

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
#[derive(Debug, Clone, Copy, Default)] pub struct Double;
#[doc(hidden)] pub type Float8 = Double;

/// The numeric SQL type.
///
/// This type does not currently have any corresponding Rust types. On SQLite,
/// [`Double`](struct.Double.html) should be used instead.
#[derive(Debug, Clone, Copy, Default)] pub struct Numeric;

#[cfg(not(feature="postgres"))]
impl NotNull for Numeric {}

/// The text SQL type.
///
/// On all backends strings must be valid UTF-8.
/// On PostgreSQL strings must not include nul bytes.
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
#[derive(Debug, Clone, Copy, Default)] pub struct Text;
pub type VarChar = Text;
#[doc(hidden)] pub type Varchar = VarChar;

/// The binary SQL type.
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
#[derive(Debug, Clone, Copy, Default)] pub struct Binary;

/// The date SQL type.
///
/// This type is currently only implemented for PostgreSQL.
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
#[derive(Debug, Clone, Copy, Default)] pub struct Date;

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
#[derive(Debug, Clone, Copy, Default)] pub struct Interval;

/// The time SQL type.
///
/// This type is currently only implemented for PostgreSQL.
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
#[derive(Debug, Clone, Copy, Default)] pub struct Time;

/// The timestamp/datetime SQL type.
///
/// This type is currently only implemented for PostgreSQL.
///
/// ### [`ToSql`](/diesel/types/trait.ToSql.html) impls
///
/// - [`std::time::SystemTime`][SystemTime]
/// - [`chrono::NaiveDateTime`][NaiveDateTime] with `feature = "chrono"`
///
/// ### [`FromSql`](/diesel/types/trait.FromSql.html) impls
///
/// - [`std::time::SystemTime`][SystemTime]
/// - [`chrono::NaiveDateTime`][NaiveDateTime] with `feature = "chrono"`
///
/// [SystemTime]: https://doc.rust-lang.org/nightly/std/time/struct.SystemTime.html
/// [NaiveDateTime]: /chrono/naive/datetime/struct.NaiveDateTime.html
#[derive(Debug, Clone, Copy, Default)] pub struct Timestamp;

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
#[derive(Debug, Clone, Copy, Default)] pub struct Nullable<ST: NotNull>(ST);

#[cfg(feature = "postgres")]
pub use pg::types::sql_types::*;

pub trait HasSqlType<ST>: TypeMetadata {
    fn metadata() -> Self::TypeMetadata;

    fn row_metadata(out: &mut Vec<Self::TypeMetadata>) {
        out.push(Self::metadata())
    }
}

pub trait NotNull {
}

pub trait IntoNullable {
    type Nullable;
}

impl<T: NotNull> IntoNullable for T {
    type Nullable = Nullable<T>;
}

impl<T: NotNull> IntoNullable for Nullable<T> {
    type Nullable = Nullable<T>;
}

/// How to deserialize a single field of a given type. The input will always be
/// the binary representation, not the text.
pub trait FromSql<A, DB: Backend + HasSqlType<A>>: Sized {
    fn from_sql(bytes: Option<&DB::RawValue>) -> Result<Self, Box<Error+Send+Sync>>;
}

/// How to deserialize multiple fields, with a known type. This type is
/// implemented for tuples of various sizes.
pub trait FromSqlRow<A, DB: Backend + HasSqlType<A>>: Sized {
    fn build_from_row<T: Row<DB>>(row: &mut T) -> Result<Self, Box<Error+Send+Sync>>;
}

#[cfg(feature = "unstable")]
impl<T, ST, DB> FromSqlRow<Nullable<ST>, DB> for Option<T> where
    T: FromSqlRow<ST, DB>,
    DB: Backend + HasSqlType<ST>,
    ST: NotNull,
{
    default fn build_from_row<R: Row<DB>>(row: &mut R) -> Result<Self, Box<Error+Send+Sync>> {
        if row.next_is_null(1) {
            row.take();
            Ok(None)
        } else {
            T::build_from_row(row).map(Some)
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
/// Tiny enum to make the return type of `ToSql` more descriptive
pub enum IsNull {
    Yes,
    No,
}

/// Serializes a single value to be sent to the database. The output will be
/// included as a bind parameter, and is expected to be the binary format, not
/// text.
pub trait ToSql<A, DB: Backend + HasSqlType<A>> {
    fn to_sql<W: Write>(&self, out: &mut W) -> Result<IsNull, Box<Error+Send+Sync>>;
}

impl<'a, A, T, DB> ToSql<A, DB> for &'a T where
    DB: Backend + HasSqlType<A>,
    T: ToSql<A, DB>,
{
    fn to_sql<W: Write>(&self, out: &mut W) -> Result<IsNull, Box<Error+Send+Sync>> {
        (*self).to_sql(out)
    }
}
