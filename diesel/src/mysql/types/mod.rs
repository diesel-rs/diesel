//! MySQL specific types

pub(super) mod date_and_time;
#[cfg(feature = "serde_json")]
mod json;
mod numeric;
mod primitives;

use crate::deserialize::{self, FromSql};
use crate::mysql::{Mysql, MysqlType, MysqlValue};
use crate::query_builder::QueryId;
use crate::serialize::{self, IsNull, Output, ToSql};
use crate::sql_types::*;
use crate::sql_types::{self, ops::*};
use byteorder::{NativeEndian, WriteBytesExt};

#[cfg(feature = "mysql_backend")]
impl ToSql<TinyInt, Mysql> for i8 {
    fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, Mysql>) -> serialize::Result {
        out.write_i8(*self).map(|_| IsNull::No).map_err(Into::into)
    }
}

#[cfg(feature = "mysql_backend")]
impl FromSql<TinyInt, Mysql> for i8 {
    fn from_sql(value: MysqlValue<'_>) -> deserialize::Result<Self> {
        let bytes = value.as_bytes();
        Ok(bytes[0] as i8)
    }
}

/// Represents the MySQL unsigned type.
#[derive(Debug, Clone, Copy, Default, SqlType, QueryId)]
#[cfg(feature = "mysql_backend")]
pub struct Unsigned<ST: 'static>(ST);

impl<T> Add for Unsigned<T>
where
    T: Add,
{
    type Rhs = Unsigned<T::Rhs>;
    type Output = Unsigned<T::Output>;
}

impl<T> Sub for Unsigned<T>
where
    T: Sub,
{
    type Rhs = Unsigned<T::Rhs>;
    type Output = Unsigned<T::Output>;
}

impl<T> Mul for Unsigned<T>
where
    T: Mul,
{
    type Rhs = Unsigned<T::Rhs>;
    type Output = Unsigned<T::Output>;
}

impl<T> Div for Unsigned<T>
where
    T: Div,
{
    type Rhs = Unsigned<T::Rhs>;
    type Output = Unsigned<T::Output>;
}

#[cfg(feature = "mysql_backend")]
impl ToSql<Unsigned<TinyInt>, Mysql> for u8 {
    fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, Mysql>) -> serialize::Result {
        ToSql::<TinyInt, Mysql>::to_sql(&(*self as i8), &mut out.reborrow())
    }
}

#[cfg(feature = "mysql_backend")]
impl FromSql<Unsigned<TinyInt>, Mysql> for u8 {
    fn from_sql(bytes: MysqlValue<'_>) -> deserialize::Result<Self> {
        let signed: i8 = FromSql::<TinyInt, Mysql>::from_sql(bytes)?;
        Ok(signed as u8)
    }
}

#[cfg(feature = "mysql_backend")]
impl ToSql<Unsigned<SmallInt>, Mysql> for u16 {
    fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, Mysql>) -> serialize::Result {
        ToSql::<SmallInt, Mysql>::to_sql(&(*self as i16), &mut out.reborrow())
    }
}

#[cfg(feature = "mysql_backend")]
impl FromSql<Unsigned<SmallInt>, Mysql> for u16 {
    fn from_sql(bytes: MysqlValue<'_>) -> deserialize::Result<Self> {
        let signed: i32 = FromSql::<Integer, Mysql>::from_sql(bytes)?;
        Ok(signed as u16)
    }
}

#[cfg(feature = "mysql_backend")]
impl ToSql<Unsigned<Integer>, Mysql> for u32 {
    fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, Mysql>) -> serialize::Result {
        ToSql::<Integer, Mysql>::to_sql(&(*self as i32), &mut out.reborrow())
    }
}

#[cfg(feature = "mysql_backend")]
impl FromSql<Unsigned<Integer>, Mysql> for u32 {
    fn from_sql(bytes: MysqlValue<'_>) -> deserialize::Result<Self> {
        let signed: i64 = FromSql::<BigInt, Mysql>::from_sql(bytes)?;
        Ok(signed as u32)
    }
}

#[cfg(feature = "mysql_backend")]
impl ToSql<Unsigned<BigInt>, Mysql> for u64 {
    fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, Mysql>) -> serialize::Result {
        ToSql::<BigInt, Mysql>::to_sql(&(*self as i64), &mut out.reborrow())
    }
}

#[cfg(feature = "mysql_backend")]
impl FromSql<Unsigned<BigInt>, Mysql> for u64 {
    fn from_sql(bytes: MysqlValue<'_>) -> deserialize::Result<Self> {
        let signed: i64 = FromSql::<BigInt, Mysql>::from_sql(bytes)?;
        Ok(signed as u64)
    }
}

#[cfg(feature = "mysql_backend")]
impl ToSql<Bool, Mysql> for bool {
    fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, Mysql>) -> serialize::Result {
        let int_value = i32::from(*self);
        <i32 as ToSql<Integer, Mysql>>::to_sql(&int_value, &mut out.reborrow())
    }
}

#[cfg(feature = "mysql_backend")]
impl FromSql<Bool, Mysql> for bool {
    fn from_sql(bytes: MysqlValue<'_>) -> deserialize::Result<Self> {
        Ok(bytes.as_bytes().iter().any(|x| *x != 0))
    }
}

#[cfg(feature = "mysql_backend")]
impl ToSql<sql_types::SmallInt, Mysql> for i16 {
    fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, Mysql>) -> serialize::Result {
        out.write_i16::<NativeEndian>(*self)
            .map(|_| IsNull::No)
            .map_err(|e| Box::new(e) as Box<_>)
    }
}

#[cfg(feature = "mysql_backend")]
impl ToSql<sql_types::Integer, Mysql> for i32 {
    fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, Mysql>) -> serialize::Result {
        out.write_i32::<NativeEndian>(*self)
            .map(|_| IsNull::No)
            .map_err(|e| Box::new(e) as Box<_>)
    }
}

#[cfg(feature = "mysql_backend")]
impl ToSql<sql_types::BigInt, Mysql> for i64 {
    fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, Mysql>) -> serialize::Result {
        out.write_i64::<NativeEndian>(*self)
            .map(|_| IsNull::No)
            .map_err(|e| Box::new(e) as Box<_>)
    }
}

#[cfg(feature = "mysql_backend")]
impl ToSql<sql_types::Double, Mysql> for f64 {
    fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, Mysql>) -> serialize::Result {
        out.write_f64::<NativeEndian>(*self)
            .map(|_| IsNull::No)
            .map_err(|e| Box::new(e) as Box<_>)
    }
}

#[cfg(feature = "mysql_backend")]
impl ToSql<sql_types::Float, Mysql> for f32 {
    fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, Mysql>) -> serialize::Result {
        out.write_f32::<NativeEndian>(*self)
            .map(|_| IsNull::No)
            .map_err(|e| Box::new(e) as Box<_>)
    }
}

#[cfg(feature = "mysql_backend")]
impl HasSqlType<Unsigned<TinyInt>> for Mysql {
    fn metadata(_lookup: &mut ()) -> MysqlType {
        MysqlType::UnsignedTiny
    }
}

#[cfg(feature = "mysql_backend")]
impl HasSqlType<Unsigned<SmallInt>> for Mysql {
    fn metadata(_lookup: &mut ()) -> MysqlType {
        MysqlType::UnsignedShort
    }
}

#[cfg(feature = "mysql_backend")]
impl HasSqlType<Unsigned<Integer>> for Mysql {
    fn metadata(_lookup: &mut ()) -> MysqlType {
        MysqlType::UnsignedLong
    }
}

#[cfg(feature = "mysql_backend")]
impl HasSqlType<Unsigned<BigInt>> for Mysql {
    fn metadata(_lookup: &mut ()) -> MysqlType {
        MysqlType::UnsignedLongLong
    }
}

/// Represents the MySQL datetime type.
///
/// ### [`ToSql`] impls
///
/// - [`chrono::NaiveDateTime`] with `feature = "chrono"`
/// - [`time::PrimitiveDateTime`] with `feature = "time"`
/// - [`time::OffsetDateTime`] with `feature = "time"`
///
/// ### [`FromSql`] impls
///
/// - [`chrono::NaiveDateTime`] with `feature = "chrono"`
/// - [`time::PrimitiveDateTime`] with `feature = "time"`
/// - [`time::OffsetDateTime`] with `feature = "time"`
///
/// [`ToSql`]: crate::serialize::ToSql
/// [`FromSql`]: crate::deserialize::FromSql
#[cfg_attr(
    feature = "chrono",
    doc = " [`chrono::NaiveDateTime`]: chrono::naive::NaiveDateTime"
)]
#[cfg_attr(
    not(feature = "chrono"),
    doc = " [`chrono::NaiveDateTime`]: https://docs.rs/chrono/0.4.19/chrono/naive/struct.NaiveDateTime.html"
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
#[derive(Debug, Clone, Copy, Default, QueryId, SqlType)]
#[diesel(mysql_type(name = "DateTime"))]
#[cfg(feature = "mysql_backend")]
pub struct Datetime;
