//! MySQL specific types
use byteorder::WriteBytesExt;
use std::io::Write;

use deserialize::{self, FromSql};
use mysql_like::MysqlLikeBackend;
use serialize::{self, IsNull, Output, ToSql};
use sql_types::*;

impl<MysqlLike: MysqlLikeBackend> ToSql<TinyInt, MysqlLike> for i8 {
    fn to_sql<W: Write>(&self, out: &mut Output<W, MysqlLike>) -> serialize::Result {
        out.write_i8(*self).map(|_| IsNull::No).map_err(Into::into)
    }
}

impl<MysqlLike: MysqlLikeBackend> FromSql<TinyInt, MysqlLike> for i8 {
    fn from_sql(bytes: Option<&[u8]>) -> deserialize::Result<Self> {
        let bytes = not_none!(bytes);
        Ok(bytes[0] as i8)
    }
}

/// Represents the MySQL unsigned type.
#[derive(Debug, Clone, Copy, Default, SqlType, QueryId)]
pub struct Unsigned<ST>(ST);

impl<MysqlLike: MysqlLikeBackend> ToSql<Unsigned<TinyInt>, MysqlLike> for u8 {
    fn to_sql<W: Write>(&self, out: &mut Output<W, MysqlLike>) -> serialize::Result {
        ToSql::<TinyInt, MysqlLike>::to_sql(&(*self as i8), out)
    }
}

impl<MysqlLike: MysqlLikeBackend> FromSql<Unsigned<TinyInt>, MysqlLike> for u8 {
    fn from_sql(bytes: Option<&[u8]>) -> deserialize::Result<Self> {
        let signed: i8 = FromSql::<TinyInt, MysqlLike>::from_sql(bytes)?;
        Ok(signed as u8)
    }
}

impl<MysqlLike: MysqlLikeBackend> ToSql<Unsigned<SmallInt>, MysqlLike> for u16 {
    fn to_sql<W: Write>(&self, out: &mut Output<W, MysqlLike>) -> serialize::Result {
        ToSql::<SmallInt, MysqlLike>::to_sql(&(*self as i16), out)
    }
}

impl<MysqlLike: MysqlLikeBackend> FromSql<Unsigned<SmallInt>, MysqlLike> for u16 {
    fn from_sql(bytes: Option<&[u8]>) -> deserialize::Result<Self> {
        let signed: i16 = FromSql::<SmallInt, MysqlLike>::from_sql(bytes)?;
        Ok(signed as u16)
    }
}

impl<MysqlLike: MysqlLikeBackend> ToSql<Unsigned<Integer>, MysqlLike> for u32 {
    fn to_sql<W: Write>(&self, out: &mut Output<W, MysqlLike>) -> serialize::Result {
        ToSql::<Integer, MysqlLike>::to_sql(&(*self as i32), out)
    }
}

impl<MysqlLike: MysqlLikeBackend> FromSql<Unsigned<Integer>, MysqlLike> for u32 {
    fn from_sql(bytes: Option<&[u8]>) -> deserialize::Result<Self> {
        let signed: i32 = FromSql::<Integer, MysqlLike>::from_sql(bytes)?;
        Ok(signed as u32)
    }
}

impl<MysqlLike: MysqlLikeBackend> ToSql<Unsigned<BigInt>, MysqlLike> for u64 {
    fn to_sql<W: Write>(&self, out: &mut Output<W, MysqlLike>) -> serialize::Result {
        ToSql::<BigInt, MysqlLike>::to_sql(&(*self as i64), out)
    }
}

impl<MysqlLike: MysqlLikeBackend> FromSql<Unsigned<BigInt>, MysqlLike> for u64 {
    fn from_sql(bytes: Option<&[u8]>) -> deserialize::Result<Self> {
        let signed: i64 = FromSql::<BigInt, MysqlLike>::from_sql(bytes)?;
        Ok(signed as u64)
    }
}

impl<MysqlLike: MysqlLikeBackend> ToSql<Bool, MysqlLike> for bool {
    fn to_sql<W: Write>(&self, out: &mut Output<W, MysqlLike>) -> serialize::Result {
        let int_value = if *self { 1 } else { 0 };
        <i32 as ToSql<Integer, MysqlLike>>::to_sql(&int_value, out)
    }
}

impl<MysqlLike: MysqlLikeBackend> FromSql<Bool, MysqlLike> for bool {
    fn from_sql(bytes: Option<&[u8]>) -> deserialize::Result<Self> {
        Ok(not_none!(bytes).iter().any(|x| *x != 0))
    }
}

impl<MysqlLike, ST> HasSqlType<Unsigned<ST>> for MysqlLike
where
    MysqlLike: MysqlLikeBackend + HasSqlType<ST>,
{
    fn metadata(lookup: &MysqlLike::MetadataLookup) -> <MysqlLike as TypeMetadata>::TypeMetadata {
        <MysqlLike as HasSqlType<ST>>::metadata(lookup)
    }

    fn is_signed() -> IsSigned {
        IsSigned::Unsigned
    }
}
