use crate::deserialize::{self, FromSql};
use crate::mysql::{Mysql, MysqlValue, NumericRepresentation};
use crate::result::Error::DeserializationError;
use crate::sql_types::{BigInt, Binary, Double, Float, Integer, SmallInt, Text};
use std::convert::TryInto;
use std::error::Error;
use std::str::{self, FromStr};

fn decimal_to_integer<T>(bytes: &[u8]) -> deserialize::Result<T>
where
    T: FromStr,
    T::Err: Error + Send + Sync + 'static,
{
    let string = str::from_utf8(bytes)?;
    let mut splited = string.split('.');
    let integer_portion = splited.next().unwrap_or_default();
    let _decimal_portion = splited.next().unwrap_or_default();
    if splited.next().is_some() {
        Err(format!("Invalid decimal format: {:?}", string).into())
    } else {
        Ok(integer_portion.parse()?)
    }
}

fn f32_to_i64(f: f32) -> deserialize::Result<i64> {
    use std::i64;

    if f <= i64::MAX as f32 && f >= i64::MIN as f32 {
        Ok(f.trunc() as i64)
    } else {
        Err(Box::new(DeserializationError(
            "Numeric overflow/underflow occurred".into(),
        )) as _)
    }
}

fn f64_to_i64(f: f64) -> deserialize::Result<i64> {
    use std::i64;

    if f <= i64::MAX as f64 && f >= i64::MIN as f64 {
        Ok(f.trunc() as i64)
    } else {
        Err(Box::new(DeserializationError(
            "Numeric overflow/underflow occurred".into(),
        )) as _)
    }
}

impl FromSql<SmallInt, Mysql> for i16 {
    fn from_sql(value: MysqlValue<'_>) -> deserialize::Result<Self> {
        match value.numeric_value()? {
            NumericRepresentation::Tiny(x) => Ok(x.into()),
            NumericRepresentation::Small(x) => Ok(x),
            NumericRepresentation::Medium(x) => x.try_into().map_err(|_| {
                Box::new(DeserializationError(
                    "Numeric overflow/underflow occurred".into(),
                )) as _
            }),
            NumericRepresentation::Big(x) => x.try_into().map_err(|_| {
                Box::new(DeserializationError(
                    "Numeric overflow/underflow occured".into(),
                )) as _
            }),
            NumericRepresentation::Float(x) => f32_to_i64(x)?.try_into().map_err(|_| {
                Box::new(DeserializationError(
                    "Numeric overflow/underflow occured".into(),
                )) as _
            }),
            NumericRepresentation::Double(x) => f64_to_i64(x)?.try_into().map_err(|_| {
                Box::new(DeserializationError(
                    "Numeric overflow/underflow occured".into(),
                )) as _
            }),
            NumericRepresentation::Decimal(bytes) => decimal_to_integer(bytes),
        }
    }
}

impl FromSql<Integer, Mysql> for i32 {
    fn from_sql(value: MysqlValue<'_>) -> deserialize::Result<Self> {
        match value.numeric_value()? {
            NumericRepresentation::Tiny(x) => Ok(x.into()),
            NumericRepresentation::Small(x) => Ok(x.into()),
            NumericRepresentation::Medium(x) => Ok(x),
            NumericRepresentation::Big(x) => x.try_into().map_err(|_| {
                Box::new(DeserializationError(
                    "Numeric overflow/underflow occured".into(),
                )) as _
            }),
            NumericRepresentation::Float(x) => f32_to_i64(x).and_then(|i| {
                i.try_into().map_err(|_| {
                    Box::new(DeserializationError(
                        "Numeric overflow/underflow occured".into(),
                    )) as _
                })
            }),
            NumericRepresentation::Double(x) => f64_to_i64(x).and_then(|i| {
                i.try_into().map_err(|_| {
                    Box::new(DeserializationError(
                        "Numeric overflow/underflow occured".into(),
                    )) as _
                })
            }),
            NumericRepresentation::Decimal(bytes) => decimal_to_integer(bytes),
        }
    }
}

impl FromSql<BigInt, Mysql> for i64 {
    fn from_sql(value: MysqlValue<'_>) -> deserialize::Result<Self> {
        match value.numeric_value()? {
            NumericRepresentation::Tiny(x) => Ok(x.into()),
            NumericRepresentation::Small(x) => Ok(x.into()),
            NumericRepresentation::Medium(x) => Ok(x.into()),
            NumericRepresentation::Big(x) => Ok(x),
            NumericRepresentation::Float(x) => f32_to_i64(x),
            NumericRepresentation::Double(x) => f64_to_i64(x),
            NumericRepresentation::Decimal(bytes) => decimal_to_integer(bytes),
        }
    }
}

impl FromSql<Float, Mysql> for f32 {
    fn from_sql(value: MysqlValue<'_>) -> deserialize::Result<Self> {
        match value.numeric_value()? {
            NumericRepresentation::Tiny(x) => Ok(x.into()),
            NumericRepresentation::Small(x) => Ok(x.into()),
            NumericRepresentation::Medium(x) => Ok(x as Self),
            NumericRepresentation::Big(x) => Ok(x as Self),
            NumericRepresentation::Float(x) => Ok(x),
            NumericRepresentation::Double(x) => Ok(x as Self),
            NumericRepresentation::Decimal(bytes) => Ok(str::from_utf8(bytes)?.parse()?),
        }
    }
}

impl FromSql<Double, Mysql> for f64 {
    fn from_sql(value: MysqlValue<'_>) -> deserialize::Result<Self> {
        match value.numeric_value()? {
            NumericRepresentation::Tiny(x) => Ok(x.into()),
            NumericRepresentation::Small(x) => Ok(x.into()),
            NumericRepresentation::Medium(x) => Ok(x.into()),
            NumericRepresentation::Big(x) => Ok(x as Self),
            NumericRepresentation::Float(x) => Ok(x.into()),
            NumericRepresentation::Double(x) => Ok(x),
            NumericRepresentation::Decimal(bytes) => Ok(str::from_utf8(bytes)?.parse()?),
        }
    }
}

impl FromSql<Text, Mysql> for String {
    fn from_sql(value: MysqlValue<'_>) -> deserialize::Result<Self> {
        String::from_utf8(value.as_bytes().into()).map_err(Into::into)
    }
}

impl FromSql<Binary, Mysql> for Vec<u8> {
    fn from_sql(value: MysqlValue<'_>) -> deserialize::Result<Self> {
        Ok(value.as_bytes().into())
    }
}
