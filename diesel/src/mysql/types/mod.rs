//! MySQL specific types
extern crate mysqlclient_sys as ffi;

#[cfg(feature = "chrono")]
mod date_and_time;
mod numeric;

use byteorder::WriteBytesExt;
use std::io::Write;
use std::os::raw as libc;

use deserialize::{self, FromSql};
use mysql::{Mysql, MysqlType};
use serialize::{self, IsNull, Output, ToSql};
use sql_types::*;

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub(crate) struct MYSQL_TIME {
    pub year: libc::c_uint,
    pub month: libc::c_uint,
    pub day: libc::c_uint,
    pub hour: libc::c_uint,
    pub minute: libc::c_uint,
    pub second: libc::c_uint,
    pub second_part: libc::c_ulong,
    pub neg: bool,
    pub time_type: ffi::enum_mysql_timestamp_type,
    pub time_zone_displacement: libc::c_int,
}

impl From<ffi::MYSQL_TIME> for MYSQL_TIME {
    fn from(item: ffi::MYSQL_TIME) -> Self {
        let ffi::MYSQL_TIME {
            year,
            month,
            day,
            hour,
            minute,
            second,
            second_part,
            neg,
            time_type,
        } = item;

        MYSQL_TIME {
            year,
            month,
            day,
            hour,
            minute,
            second,
            second_part,
            neg: neg != 0,
            time_type,
            time_zone_displacement: 0,
        }
    }
}

impl From<MYSQL_TIME> for ffi::MYSQL_TIME {
    fn from(item: MYSQL_TIME) -> Self {
        let MYSQL_TIME {
            year,
            month,
            day,
            hour,
            minute,
            second,
            second_part,
            neg,
            time_type,
            time_zone_displacement: _,
        } = item;

        ffi::MYSQL_TIME {
            year,
            month,
            day,
            hour,
            minute,
            second,
            second_part,
            neg: if neg { 1 } else { 0 },
            time_type,
        }
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
