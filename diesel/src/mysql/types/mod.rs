//! MySQL specific types

#[cfg(feature = "chrono")]
mod date_and_time;
mod numeric;

use byteorder::WriteBytesExt;
use std::io::Write;

use deserialize::{self, FromSql};
use mysql::{Mysql, MysqlType};
use serialize::{self, IsNull, Output, ToSql};
use sql_types::*;

impl ToSql<Tinyint, Mysql> for i8 {
    fn to_sql<W: Write>(&self, out: &mut Output<W, Mysql>) -> serialize::Result {
        out.write_i8(*self).map(|_| IsNull::No).map_err(Into::into)
    }
}

impl FromSql<Tinyint, Mysql> for i8 {
    fn from_sql(bytes: Option<&[u8]>) -> deserialize::Result<Self> {
        let bytes = not_none!(bytes);
        Ok(bytes[0] as i8)
    }
}

/// Represents the MySQL unsigned type.
#[derive(Debug, Clone, Copy, Default, SqlType, QueryId)]
pub struct Unsigned<ST>(ST);

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

impl HasSqlType<Unsigned<Tinyint>> for Mysql {
    fn metadata(_: &()) -> MysqlType {
        MysqlType::UnsignedTiny
    }
}

impl HasSqlType<Unsigned<SmallInt>> for Mysql {
    fn metadata(_: &()) -> MysqlType {
        MysqlType::UnsignedShort
    }
}

impl HasSqlType<Unsigned<Integer>> for Mysql {
    fn metadata(_: &()) -> MysqlType {
        MysqlType::UnsignedLong
    }
}

impl HasSqlType<Unsigned<BigInt>> for Mysql {
    fn metadata(_: &()) -> MysqlType {
        MysqlType::UnsignedLongLong
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
