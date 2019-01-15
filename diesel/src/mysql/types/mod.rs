//! MySQL specific types

#[cfg(feature = "chrono")]
mod date_and_time;
mod numeric;

use byteorder::{NativeEndian, ReadBytesExt, WriteBytesExt};
use std::io::Write;
use std::error::Error;

use deserialize::{self, FromSql};
use mysql::{Mysql, MysqlType};
use serialize::{self, IsNull, Output, ToSql};
use sql_types::*;

impl FromSql<Text, Mysql> for *const str {
    fn from_sql(bytes: Option<&[u8]>) -> deserialize::Result<Self> {
        use std::str;
        let string = str::from_utf8(not_none!(bytes))?;
        Ok(string as *const _)
    }
}

impl ToSql<TinyInt, Mysql> for i8 {
    fn to_sql<W: Write>(&self, out: &mut Output<W, Mysql>) -> serialize::Result {
        out.write_i8(*self).map(|_| IsNull::No).map_err(Into::into)
    }
}

impl FromSql<TinyInt, Mysql> for i8 {
    fn from_sql(bytes: Option<&[u8]>) -> deserialize::Result<Self> {
        let bytes = not_none!(bytes);
        Ok(bytes[0] as i8)
    }
}

impl FromSql<SmallInt, Mysql> for i16 {
    fn from_sql(bytes: Option<&[u8]>) -> deserialize::Result<Self> {
        let mut bytes = not_none!(bytes);
        debug_assert!(
            bytes.len() <= 2,
            "Received more than 2 bytes decoding i16. \
             Was an Integer expression accidentally identified as SmallInt?"
        );
        debug_assert!(
            bytes.len() >= 2,
            "Received fewer than 2 bytes decoding i16. \
             Was an expression of a different type accidentally identified \
             as SmallInt?"
        );
        bytes
            .read_i16::<NativeEndian>()
            .map_err(|e| Box::new(e) as Box<Error + Send + Sync>)
    }
}

impl FromSql<Integer, Mysql> for i32 {
    fn from_sql(bytes: Option<&[u8]>) -> deserialize::Result<Self> {
        let mut bytes = not_none!(bytes);
        debug_assert!(
            bytes.len() <= 4,
            "Received more than 4 bytes decoding i32. \
             Was a BigInteger expression accidentally identified as Integer?"
        );
        debug_assert!(
            bytes.len() >= 4,
            "Received fewer than 4 bytes decoding i32. \
             Was a SmallInteger expression accidentally identified as Integer?"
        );
        bytes
            .read_i32::<NativeEndian>()
            .map_err(|e| Box::new(e) as Box<Error + Send + Sync>)
    }
}

impl FromSql<BigInt, Mysql> for i64 {
    fn from_sql(bytes: Option<&[u8]>) -> deserialize::Result<Self> {
        let mut bytes = not_none!(bytes);
        debug_assert!(
            bytes.len() <= 8,
            "Received more than 8 bytes decoding i64. \
             Was an expression of a different type misidentified as BigInteger?"
        );
        debug_assert!(
            bytes.len() >= 8,
            "Received fewer than 8 bytes decoding i64. \
             Was an Integer expression misidentified as BigInteger?"
        );
        bytes
            .read_i64::<NativeEndian>()
            .map_err(|e| Box::new(e) as Box<Error + Send + Sync>)
    }
}


/// Represents the MySQL unsigned type.
#[derive(Debug, Clone, Copy, Default, SqlType, QueryId)]
pub struct Unsigned<ST>(ST);

impl ToSql<Unsigned<TinyInt>, Mysql> for u8 {
    fn to_sql<W: Write>(&self, out: &mut Output<W, Mysql>) -> serialize::Result {
        ToSql::<TinyInt, Mysql>::to_sql(&(*self as i8), out)
    }
}

impl FromSql<Unsigned<TinyInt>, Mysql> for u8 {
    fn from_sql(bytes: Option<&[u8]>) -> deserialize::Result<Self> {
        let signed: i8 = FromSql::<TinyInt, Mysql>::from_sql(bytes)?;
        Ok(signed as u8)
    }
}

impl ToSql<Unsigned<SmallInt>, Mysql> for u16 {
    fn to_sql<W: Write>(&self, out: &mut Output<W, Mysql>) -> serialize::Result {
        ToSql::<SmallInt, Mysql>::to_sql(&(*self as i16), out)
    }
}

impl FromSql<Unsigned<SmallInt>, Mysql> for u16 {
    fn from_sql(bytes: Option<&[u8]>) -> deserialize::Result<Self> {
        let signed: i16 = FromSql::<SmallInt, Mysql>::from_sql(bytes)?;
        Ok(signed as u16)
    }
}

impl ToSql<Unsigned<Integer>, Mysql> for u32 {
    fn to_sql<W: Write>(&self, out: &mut Output<W, Mysql>) -> serialize::Result {
        ToSql::<Integer, Mysql>::to_sql(&(*self as i32), out)
    }
}

impl FromSql<Unsigned<Integer>, Mysql> for u32 {
    fn from_sql(bytes: Option<&[u8]>) -> deserialize::Result<Self> {
        let signed: i32 = FromSql::<Integer, Mysql>::from_sql(bytes)?;
        Ok(signed as u32)
    }
}

impl ToSql<Unsigned<BigInt>, Mysql> for u64 {
    fn to_sql<W: Write>(&self, out: &mut Output<W, Mysql>) -> serialize::Result {
        ToSql::<BigInt, Mysql>::to_sql(&(*self as i64), out)
    }
}

impl FromSql<Unsigned<BigInt>, Mysql> for u64 {
    fn from_sql(bytes: Option<&[u8]>) -> deserialize::Result<Self> {
        let signed: i64 = FromSql::<BigInt, Mysql>::from_sql(bytes)?;
        Ok(signed as u64)
    }
}

impl ToSql<Bool, Mysql> for bool {
    fn to_sql<W: Write>(&self, out: &mut Output<W, Mysql>) -> serialize::Result {
        let int_value = if *self { 1 } else { 0 };
        <i32 as ToSql<Integer, Mysql>>::to_sql(&int_value, out)
    }
}

impl FromSql<Bool, Mysql> for bool {
    fn from_sql(bytes: Option<&[u8]>) -> deserialize::Result<Self> {
        Ok(not_none!(bytes).iter().any(|x| *x != 0))
    }
}

impl FromSql<Float, Mysql> for f32 {
    fn from_sql(bytes: Option<&[u8]>) -> deserialize::Result<Self> {
        let mut bytes = not_none!(bytes);
        debug_assert!(
            bytes.len() <= 4,
            "Received more than 4 bytes while decoding \
             an f32. Was a double accidentally marked as float?"
        );
        bytes
            .read_f32::<NativeEndian>()
            .map_err(|e| Box::new(e) as Box<Error + Send + Sync>)
    }
}

impl FromSql<Double, Mysql> for f64 {
    fn from_sql(bytes: Option<&[u8]>) -> deserialize::Result<Self> {
        let mut bytes = not_none!(bytes);
        debug_assert!(
            bytes.len() <= 8,
            "Received more than 8 bytes while decoding \
             an f64. Was a numeric accidentally marked as double?"
        );
        bytes
            .read_f64::<NativeEndian>()
            .map_err(|e| Box::new(e) as Box<Error + Send + Sync>)
    }
}

impl<ST> HasSqlType<Unsigned<ST>> for Mysql
where
    Mysql: HasSqlType<ST>,
{
    fn metadata(lookup: &()) -> MysqlType {
        <Mysql as HasSqlType<ST>>::metadata(lookup)
    }

    fn is_signed() -> IsSigned {
        IsSigned::Unsigned
    }
}

/// The returned pointer is *only* valid for the lifetime to the argument of
/// `from_sql`. This impl is intended for uses where you want to write a new
/// impl in terms of `Vec<u8>`, but don't want to allocate. We have to return a
/// raw pointer instead of a reference with a lifetime due to the structure of
/// `FromSql`
impl FromSql<Binary, Mysql> for *const [u8] {
    fn from_sql(bytes: Option<&[u8]>) -> deserialize::Result<Self> {
        Ok(not_none!(bytes) as *const _)
    }
}

/// Represents the MySQL datetime type.
///
/// ### [`ToSql`] impls
///
/// - [`chrono::NaiveDateTime`] with `feature = "chrono"`
///
/// ### [`FromSql`] impls
///
/// - [`chrono::NaiveDateTime`] with `feature = "chrono"`
///
/// [`ToSql`]: ../../serialize/trait.ToSql.html
/// [`FromSql`]: ../../deserialize/trait.FromSql.html
/// [`chrono::NaiveDateTime`]: ../../../chrono/naive/struct.NaiveDateTime.html
#[derive(Debug, Clone, Copy, Default, QueryId, SqlType)]
#[mysql_type = "DateTime"]
pub struct Datetime;
