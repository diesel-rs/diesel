//! Types which represent a native SQL data type, and the conversions between
//! them and Rust primitives. Additional types can be added by other crates.
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

#[derive(Clone, Copy, Default)] pub struct Bool;

#[derive(Clone, Copy, Default)] pub struct SmallInt;
#[doc(hidden)] pub type Int2 = SmallInt;
#[derive(Clone, Copy, Default)] pub struct Integer;
#[doc(hidden)] pub type Int4 = Integer;
#[derive(Clone, Copy, Default)] pub struct BigInt;
#[doc(hidden)] pub type Int8 = BigInt;

#[derive(Clone, Copy, Default)] pub struct Float;
#[doc(hidden)] pub type Float4 = Float;
#[derive(Clone, Copy, Default)] pub struct Double;
#[doc(hidden)] pub type Float8 = Double;
#[derive(Clone, Copy, Default)] pub struct Numeric;

#[derive(Clone, Copy, Default)] pub struct Text;
pub type VarChar = Text;
#[doc(hidden)] pub type Varchar = VarChar;

#[derive(Clone, Copy, Default)] pub struct Binary;

#[derive(Clone, Copy, Default)] pub struct Date;
#[derive(Clone, Copy, Default)] pub struct Interval;
#[derive(Clone, Copy, Default)] pub struct Time;
#[derive(Clone, Copy, Default)] pub struct Timestamp;

#[derive(Clone, Copy, Default)] pub struct Nullable<T: NotNull>(T);

#[cfg(feature = "postgres")]
#[doc(inline)]
pub use pg::types::sql_types::*;

pub trait HasSqlType<ST>: TypeMetadata {
    fn metadata() -> Self::TypeMetadata;
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

// FIXME: This can be inlined once 1.9 is stable. The parser can't handle the `default` keyword on
// stable prior to that version
#[cfg(feature = "unstable")]
include!("specialization_impls.rs");

#[derive(Debug, PartialEq, Eq)]
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
