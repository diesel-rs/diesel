use std::error::Error;
use std::str::{self, FromStr};

use crate::deserialize::{self, FromSql};
use crate::mysql::{Mysql, MysqlValue};
use crate::sql_types::{BigInt, Binary, Double, Float, Integer, SmallInt, Text};

fn decimal_to_integer<T>(bytes: &[u8]) -> deserialize::Result<T>
where
    T: FromStr,
    T::Err: Error + Send + Sync + 'static,
{
    let string = str::from_utf8(bytes)?;
    let mut splited = string.split('.');
    let integer_portion = splited.next().unwrap_or_default();
    let decimal_portion = splited.next().unwrap_or_default();
    if splited.next().is_some() {
        Err(format!("Invalid decimal format: {:?}", string).into())
    } else if decimal_portion.chars().any(|c| c != '0') {
        Err(format!(
            "Tried to convert a decimal to an integer that contained /
             a non null decimal portion: {:?}",
            string
        )
        .into())
    } else {
        Ok(integer_portion.parse()?)
    }
}

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
            Decimal(bytes) => decimal_to_integer(bytes),
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
            Decimal(bytes) => decimal_to_integer(bytes),
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
            Decimal(bytes) => decimal_to_integer(bytes),
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
