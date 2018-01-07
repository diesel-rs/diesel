use std::str;

use crate::deserialize::{self, FromSql};
use crate::mysql::{Mysql, MysqlValue};
use crate::sql_types::{BigInt, Binary, Double, Float, Integer, SmallInt, Text};

impl FromSql<SmallInt, Mysql> for i16 {
    fn from_sql(value: Option<MysqlValue<'_>>) -> deserialize::Result<Self> {
        use crate::mysql::NumericRepresentation::*;

        let data = not_none!(value);
        match data.numeric_value()? {
            Tiny(x) => Ok(x.into()),
            Small(x) => Ok(x),
            Medium(x) => Ok(x as Self),
            Big(x) => Ok(x as Self),
            Float(x) => Ok(x as Self),
            Double(x) => Ok(x as Self),
            Decimal(bytes) => {
                let string = str::from_utf8(bytes)?;
                let integer_portion = string.split('.').nth(0).unwrap_or_default();
                Ok(integer_portion.parse()?)
            }
        }
    }
}

impl FromSql<Integer, Mysql> for i32 {
    fn from_sql(value: Option<MysqlValue<'_>>) -> deserialize::Result<Self> {
        use crate::mysql::NumericRepresentation::*;

        let data = not_none!(value);
        match data.numeric_value()? {
            Tiny(x) => Ok(x.into()),
            Small(x) => Ok(x.into()),
            Medium(x) => Ok(x),
            Big(x) => Ok(x as Self),
            Float(x) => Ok(x as Self),
            Double(x) => Ok(x as Self),
            Decimal(bytes) => {
                let string = str::from_utf8(bytes)?;
                let integer_portion = string.split('.').nth(0).unwrap_or_default();
                Ok(integer_portion.parse()?)
            }
        }
    }
}

impl FromSql<BigInt, Mysql> for i64 {
    fn from_sql(value: Option<MysqlValue<'_>>) -> deserialize::Result<Self> {
        use crate::mysql::NumericRepresentation::*;

        let data = not_none!(value);
        match data.numeric_value()? {
            Tiny(x) => Ok(x.into()),
            Small(x) => Ok(x.into()),
            Medium(x) => Ok(x.into()),
            Big(x) => Ok(x),
            Float(x) => Ok(x as Self),
            Double(x) => Ok(x as Self),
            Decimal(bytes) => {
                let string = str::from_utf8(bytes)?;
                let integer_portion = string.split('.').nth(0).unwrap_or_default();
                Ok(integer_portion.parse()?)
            }
        }
    }
}

impl FromSql<Float, Mysql> for f32 {
    fn from_sql(value: Option<MysqlValue<'_>>) -> deserialize::Result<Self> {
        use crate::mysql::NumericRepresentation::*;

        let data = not_none!(value);
        match data.numeric_value()? {
            Tiny(x) => Ok(x.into()),
            Small(x) => Ok(x.into()),
            Medium(x) => Ok(x as Self),
            Big(x) => Ok(x as Self),
            Float(x) => Ok(x),
            Double(x) => Ok(x as Self),
            Decimal(bytes) => Ok(str::from_utf8(bytes)?.parse()?),
        }
    }
}

impl FromSql<Double, Mysql> for f64 {
    fn from_sql(value: Option<MysqlValue<'_>>) -> deserialize::Result<Self> {
        use crate::mysql::NumericRepresentation::*;

        let data = not_none!(value);
        match data.numeric_value()? {
            Tiny(x) => Ok(x.into()),
            Small(x) => Ok(x.into()),
            Medium(x) => Ok(x.into()),
            Big(x) => Ok(x as Self),
            Float(x) => Ok(x.into()),
            Double(x) => Ok(x),
            Decimal(bytes) => Ok(str::from_utf8(bytes)?.parse()?),
        }
    }
}

impl FromSql<Text, Mysql> for String {
    fn from_sql(value: Option<MysqlValue<'_>>) -> deserialize::Result<Self> {
        let value = not_none!(value);
        String::from_utf8(value.as_bytes().into()).map_err(Into::into)
    }
}

impl FromSql<Binary, Mysql> for Vec<u8> {
    fn from_sql(value: Option<MysqlValue<'_>>) -> deserialize::Result<Self> {
        let value = not_none!(value);
        Ok(value.as_bytes().into())
    }
}
