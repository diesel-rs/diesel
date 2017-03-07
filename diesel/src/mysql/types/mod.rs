#[cfg(feature = "chrono")]
mod date_and_time;

use byteorder::{WriteBytesExt};
use mysql::{Mysql, MysqlType, backend};
use std::error::Error as StdError;
use std::io::Write;
use types::{ToSql, IsNull, FromSql, HasSqlType, Unsigned};
use backend::Backend;

impl ToSql<::types::Bool, Mysql> for bool {
    fn to_sql<W: Write>(&self, out: &mut W) -> Result<IsNull, Box<StdError+Send+Sync>> {
        let int_value = if *self {
            1
        } else {
            0
        };
        <i32 as ToSql<::types::Integer, Mysql>>::to_sql(&int_value, out)
    }
}

impl FromSql<::types::Bool, Mysql> for bool {
    fn from_sql(bytes: Option<&[u8]>) -> Result<Self, Box<StdError+Send+Sync>> {
        Ok(not_none!(bytes).iter().any(|x| *x != 0))
    }
}

impl HasSqlType<::types::Date> for Mysql {
    fn metadata() -> MysqlType {
        MysqlType::Date
    }
}

impl HasSqlType<::types::Time> for Mysql {
    fn metadata() -> MysqlType {
        MysqlType::Time
    }
}

impl HasSqlType<::types::Timestamp> for Mysql {
    fn metadata() -> MysqlType {
        MysqlType::Timestamp
    }
}

impl FromSql<Unsigned<::types::SmallInt>, Mysql> for u16 {
    fn from_sql(bytes: Option<&[u8]>) -> Result<Self, Box<StdError+Send+Sync>> {
        let value: i16 = FromSql::<::types::SmallInt, Mysql>::from_sql(bytes)?;
        Ok(value as u16)
    }
}

impl ToSql<Unsigned<::types::SmallInt>, Mysql> for u16 {
    fn to_sql<W: Write>(&self, out: &mut W) -> Result<IsNull, Box<StdError+Send+Sync>> {
        out.write_u16::<<backend::Mysql as Backend>::ByteOrder>(*self)
            .map(|_| IsNull::No)
            .map_err(|e| Box::new(e) as Box<StdError+Send+Sync>)
    }
}

impl FromSql<Unsigned<::types::Integer>, Mysql> for u32 {
    fn from_sql(bytes: Option<&[u8]>) -> Result<Self, Box<StdError+Send+Sync>> {
        let value: i32 = FromSql::<::types::Integer, Mysql>::from_sql(bytes)?;
        Ok(value as u32)
    }
}

impl ToSql<Unsigned<::types::Integer>, Mysql> for u32 {
    fn to_sql<W: Write>(&self, out: &mut W) -> Result<IsNull, Box<StdError+Send+Sync>> {
        out.write_u32::<<backend::Mysql as Backend>::ByteOrder>(*self)
            .map(|_| IsNull::No)
            .map_err(|e| Box::new(e) as Box<StdError+Send+Sync>)
    }
}

impl FromSql<Unsigned<::types::BigInt>, Mysql> for u64 {
    fn from_sql(bytes: Option<&[u8]>) -> Result<Self, Box<StdError+Send+Sync>> {
        let value: i64 = FromSql::<::types::BigInt, Mysql>::from_sql(bytes)?;
        Ok(value as u64)
    }
}

impl ToSql<Unsigned<::types::BigInt>, Mysql> for u64 {
    fn to_sql<W: Write>(&self, out: &mut W) -> Result<IsNull, Box<StdError+Send+Sync>> {
        out.write_u64::<<backend::Mysql as Backend>::ByteOrder>(*self)
            .map(|_| IsNull::No)
            .map_err(|e| Box::new(e) as Box<StdError+Send+Sync>)
    }
}

