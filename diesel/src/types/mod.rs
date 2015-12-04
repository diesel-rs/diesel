//! Types which represent a native SQL data type, and the conversions between
//! them and Rust primitives. Additional types can be added by other crates.
pub mod ops;
mod ord;
mod impls;

#[doc(hidden)]
pub mod structs {
    pub mod data_types {
        //! Structs to represent the primitive equivalent of SQL types where
        //! there is no existing Rust primitive, or where using it would be
        //! confusing (such as date and time types)
        pub use super::super::impls::date_and_time::{PgTimestamp, PgDate, PgTime, PgInterval};
        pub use super::super::impls::floats::PgNumeric;
    }
}

/// Marker trait for types which can be compared for ordering.
pub use self::ord::SqlOrd;

use row::Row;
use std::error::Error;
use std::io::Write;

#[derive(Clone, Copy, Default)] pub struct Bool;

pub type SmallSerial = SmallInt;
pub type Serial = Integer;
pub type BigSerial = BigInt;

#[derive(Clone, Copy, Default)] pub struct SmallInt;
#[derive(Clone, Copy, Default)] pub struct Integer;
#[derive(Clone, Copy, Default)] pub struct BigInt;

#[derive(Clone, Copy, Default)] pub struct Float;
#[derive(Clone, Copy, Default)] pub struct Double;
#[derive(Clone, Copy, Default)] pub struct Numeric;

#[derive(Clone, Copy, Default)] pub struct Oid;

#[derive(Clone, Copy, Default)] pub struct VarChar;
#[derive(Clone, Copy, Default)] pub struct Text;

#[derive(Clone, Copy, Default)] pub struct Binary;

#[derive(Clone, Copy, Default)] pub struct Date;
#[derive(Clone, Copy, Default)] pub struct Interval;
#[derive(Clone, Copy, Default)] pub struct Time;
#[derive(Clone, Copy, Default)] pub struct Timestamp;

#[derive(Clone, Copy, Default)] pub struct Nullable<T: NativeSqlType>(T);
#[derive(Clone, Copy, Default)] pub struct Array<T: NativeSqlType>(T);

pub trait NativeSqlType {
    fn oid(&self) -> u32;
    fn new() -> Self where Self: Sized;
}

/// How to deserialize a single field of a given type. The input will always be
/// the binary representation, not the text.
pub trait FromSql<A: NativeSqlType>: Sized {
    fn from_sql(bytes: Option<&[u8]>) -> Result<Self, Box<Error>>;
}

/// How to deserialize multiple fields, with a known type. This type is
/// implemented for tuples of various sizes.
pub trait FromSqlRow<A: NativeSqlType>: Sized {
    fn build_from_row<T: Row>(row: &mut T) -> Result<Self, Box<Error>>;
}

impl<A, T> FromSqlRow<A> for T where
    A: NativeSqlType,
    T: FromSql<A>,
{
    fn build_from_row<R: Row>(row: &mut R) -> Result<Self, Box<Error>> {
        Self::from_sql(row.take())
    }
}

#[derive(Debug, PartialEq, Eq)]
/// Tiny enum to make the return type of `ToSql` more descriptive
pub enum IsNull {
    Yes,
    No,
}

/// Serializes a single value to be sent to the database. The output will be
/// included as a bind parameter, and is expected to be the binary format, not
/// text.
pub trait ToSql<A: NativeSqlType> {
    fn to_sql<W: Write>(&self, out: &mut W) -> Result<IsNull, Box<Error>>;
}

impl<'a, A, T> ToSql<A> for &'a T where
    A: NativeSqlType,
    T: ToSql<A>,
{
    fn to_sql<W: Write>(&self, out: &mut W) -> Result<IsNull, Box<Error>> {
        (*self).to_sql(out)
    }
}
