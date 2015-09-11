extern crate byteorder;

use self::byteorder::{ReadBytesExt, BigEndian};
use super::option::UnexpectedNullError;
use types::{NativeSqlType, FromSql};
use {Queriable, types};
use std::error::Error;

macro_rules! primitive_impls {
    ($($Source:ident -> $Target:ty),+,) => {
        $(
            impl NativeSqlType for types::$Source {}

            impl Queriable<types::$Source> for $Target {
                type Row = Self;

                fn build(row: Self::Row) -> Self {
                    row
                }
            }
        )+
    }
}

primitive_impls! {
    Bool -> bool,

    SmallSerial -> i16,
    Serial -> i32,
    BigSerial -> i64,

    SmallInt -> i16,
    Integer -> i32,
    BigInt -> i64,

    Float -> f32,
    Double -> f64,

    VarChar -> String,

    Binary -> Vec<u8>,
}

impl FromSql<types::Bool> for bool {
    fn from_sql(bytes: Option<&[u8]>) -> Result<Self, Box<Error>> {
        let bytes = not_none!(bytes);
        Ok(bytes[0] != 0)
    }
}

impl FromSql<types::SmallInt> for i16 {
    fn from_sql(bytes: Option<&[u8]>) -> Result<Self, Box<Error>> {
        let mut bytes = not_none!(bytes);
        bytes.read_i16::<BigEndian>().map_err(|e| Box::new(e) as Box<Error>)
    }
}

impl FromSql<types::SmallSerial> for i16 {
    fn from_sql(bytes: Option<&[u8]>) -> Result<Self, Box<Error>> {
        <Self as FromSql<types::SmallInt>>::from_sql(bytes)
    }
}

impl FromSql<types::Integer> for i32 {
    fn from_sql(bytes: Option<&[u8]>) -> Result<Self, Box<Error>> {
        let mut bytes = not_none!(bytes);
        bytes.read_i32::<BigEndian>().map_err(|e| Box::new(e) as Box<Error>)
    }
}

impl FromSql<types::Serial> for i32 {
    fn from_sql(bytes: Option<&[u8]>) -> Result<Self, Box<Error>> {
        <Self as FromSql<types::Integer>>::from_sql(bytes)
    }
}

impl FromSql<types::BigInt> for i64 {
    fn from_sql(bytes: Option<&[u8]>) -> Result<Self, Box<Error>> {
        let mut bytes = not_none!(bytes);
        bytes.read_i64::<BigEndian>().map_err(|e| Box::new(e) as Box<Error>)
    }
}

impl FromSql<types::BigSerial> for i64 {
    fn from_sql(bytes: Option<&[u8]>) -> Result<Self, Box<Error>> {
        <Self as FromSql<types::BigInt>>::from_sql(bytes)
    }
}

impl FromSql<types::Float> for f32 {
    fn from_sql(bytes: Option<&[u8]>) -> Result<Self, Box<Error>> {
        let mut bytes = not_none!(bytes);
        bytes.read_f32::<BigEndian>().map_err(|e| Box::new(e) as Box<Error>)
    }
}
impl FromSql<types::Double> for f64 {
    fn from_sql(bytes: Option<&[u8]>) -> Result<Self, Box<Error>> {
        let mut bytes = not_none!(bytes);
        bytes.read_f64::<BigEndian>().map_err(|e| Box::new(e) as Box<Error>)
    }
}

impl FromSql<types::VarChar> for String {
    fn from_sql(bytes: Option<&[u8]>) -> Result<Self, Box<Error>> {
        let bytes = not_none!(bytes);
        String::from_utf8(bytes.into()).map_err(|e| Box::new(e) as Box<Error>)
    }
}

impl FromSql<types::Binary> for Vec<u8> {
    fn from_sql(bytes: Option<&[u8]>) -> Result<Self, Box<Error>> {
        Ok(not_none!(bytes).into())
    }
}
