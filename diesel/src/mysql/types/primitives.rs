#[cfg(feature = "mysql_backend")]
use crate::deserialize::FromSqlRef;
use crate::deserialize::{self, FromSql};
use crate::mysql::{Mysql, MysqlValue, NumericRepresentation};
use crate::result::Error::DeserializationError;
use crate::sql_types::{BigInt, Binary, Double, Float, Integer, SmallInt, Text};
use crate::Queryable;
use core::error::Error;
use core::str::{self, FromStr};

pub(super) fn decimal_to_integer<T>(bytes: &[u8]) -> deserialize::Result<T>
where
    T: FromStr,
    T::Err: Error + Send + Sync + 'static,
{
    let string = str::from_utf8(bytes)?;
    let mut split = string.split('.');
    let integer_portion = split.next().unwrap_or_default();
    let _decimal_portion = split.next().unwrap_or_default();
    if split.next().is_some() {
        Err(format!("Invalid decimal format: {string:?}").into())
    } else {
        Ok(integer_portion.parse()?)
    }
}

pub(super) fn overflow() -> Box<dyn Error + Send + Sync> {
    Box::new(DeserializationError(
        "Numeric overflow/underflow occurred".into(),
    ))
}

/// Converts between the integer widths MySQL transmits, reporting an error
/// rather than wrapping when the value does not fit the target.
pub(super) fn narrow<T, U: TryFrom<T>>(value: T) -> deserialize::Result<U> {
    U::try_from(value).map_err(|_| overflow())
}

#[allow(clippy::cast_possible_truncation)] // that's what we want here
pub(super) fn f32_to_i64(f: f32) -> deserialize::Result<i64> {
    if f <= i64::MAX as f32 && f >= i64::MIN as f32 {
        Ok(f.trunc() as i64)
    } else {
        Err(overflow())
    }
}

#[allow(clippy::cast_possible_truncation)] // that's what we want here
pub(super) fn f64_to_i64(f: f64) -> deserialize::Result<i64> {
    if f <= i64::MAX as f64 && f >= i64::MIN as f64 {
        Ok(f.trunc() as i64)
    } else {
        Err(overflow())
    }
}

#[cfg(feature = "mysql_backend")]
impl FromSql<SmallInt, Mysql> for i16 {
    fn from_sql(value: MysqlValue<'_>) -> deserialize::Result<Self> {
        match value.numeric_value()? {
            NumericRepresentation::Tiny(x) => Ok(x.into()),
            NumericRepresentation::UnsignedTiny(x) => Ok(x.into()),
            NumericRepresentation::Small(x) => Ok(x),
            NumericRepresentation::UnsignedSmall(x) => narrow(x),
            NumericRepresentation::Medium(x) => narrow(x),
            NumericRepresentation::UnsignedMedium(x) => narrow(x),
            NumericRepresentation::Big(x) => narrow(x),
            NumericRepresentation::UnsignedBig(x) => narrow(x),
            NumericRepresentation::Float(x) => narrow(f32_to_i64(x)?),
            NumericRepresentation::Double(x) => narrow(f64_to_i64(x)?),
            NumericRepresentation::Decimal(bytes) => decimal_to_integer(bytes),
        }
    }
}

#[cfg(feature = "mysql_backend")]
impl FromSql<Integer, Mysql> for i32 {
    fn from_sql(value: MysqlValue<'_>) -> deserialize::Result<Self> {
        match value.numeric_value()? {
            NumericRepresentation::Tiny(x) => Ok(x.into()),
            NumericRepresentation::UnsignedTiny(x) => Ok(x.into()),
            NumericRepresentation::Small(x) => Ok(x.into()),
            NumericRepresentation::UnsignedSmall(x) => Ok(x.into()),
            NumericRepresentation::Medium(x) => Ok(x),
            NumericRepresentation::UnsignedMedium(x) => narrow(x),
            NumericRepresentation::Big(x) => narrow(x),
            NumericRepresentation::UnsignedBig(x) => narrow(x),
            NumericRepresentation::Float(x) => narrow(f32_to_i64(x)?),
            NumericRepresentation::Double(x) => narrow(f64_to_i64(x)?),
            NumericRepresentation::Decimal(bytes) => decimal_to_integer(bytes),
        }
    }
}

#[cfg(feature = "mysql_backend")]
impl FromSql<BigInt, Mysql> for i64 {
    fn from_sql(value: MysqlValue<'_>) -> deserialize::Result<Self> {
        match value.numeric_value()? {
            NumericRepresentation::Tiny(x) => Ok(x.into()),
            NumericRepresentation::UnsignedTiny(x) => Ok(x.into()),
            NumericRepresentation::Small(x) => Ok(x.into()),
            NumericRepresentation::UnsignedSmall(x) => Ok(x.into()),
            NumericRepresentation::Medium(x) => Ok(x.into()),
            NumericRepresentation::UnsignedMedium(x) => Ok(x.into()),
            NumericRepresentation::Big(x) => Ok(x),
            NumericRepresentation::UnsignedBig(x) => narrow(x),
            NumericRepresentation::Float(x) => f32_to_i64(x),
            NumericRepresentation::Double(x) => f64_to_i64(x),
            NumericRepresentation::Decimal(bytes) => decimal_to_integer(bytes),
        }
    }
}

#[cfg(feature = "mysql_backend")]
impl FromSql<Float, Mysql> for f32 {
    fn from_sql(value: MysqlValue<'_>) -> deserialize::Result<Self> {
        // Precision loss beyond the mantissa is intended for a float column.
        match value.numeric_value()? {
            NumericRepresentation::Tiny(x) => Ok(x.into()),
            NumericRepresentation::UnsignedTiny(x) => Ok(x.into()),
            NumericRepresentation::Small(x) => Ok(x.into()),
            NumericRepresentation::UnsignedSmall(x) => Ok(x.into()),
            NumericRepresentation::Medium(x) => Ok(x as Self),
            NumericRepresentation::UnsignedMedium(x) => Ok(x as Self),
            NumericRepresentation::Big(x) => Ok(x as Self),
            NumericRepresentation::UnsignedBig(x) => Ok(x as Self),
            NumericRepresentation::Float(x) => Ok(x),
            // there is currently no way to do this in a better way
            #[allow(clippy::cast_possible_truncation)]
            NumericRepresentation::Double(x) => Ok(x as Self),
            NumericRepresentation::Decimal(bytes) => Ok(str::from_utf8(bytes)?.parse()?),
        }
    }
}

#[cfg(feature = "mysql_backend")]
impl FromSql<Double, Mysql> for f64 {
    fn from_sql(value: MysqlValue<'_>) -> deserialize::Result<Self> {
        // Precision loss beyond the mantissa is intended for a double column.
        match value.numeric_value()? {
            NumericRepresentation::Tiny(x) => Ok(x.into()),
            NumericRepresentation::UnsignedTiny(x) => Ok(x.into()),
            NumericRepresentation::Small(x) => Ok(x.into()),
            NumericRepresentation::UnsignedSmall(x) => Ok(x.into()),
            NumericRepresentation::Medium(x) => Ok(x.into()),
            NumericRepresentation::UnsignedMedium(x) => Ok(x.into()),
            NumericRepresentation::Big(x) => Ok(x as Self),
            NumericRepresentation::UnsignedBig(x) => Ok(x as Self),
            NumericRepresentation::Float(x) => Ok(x.into()),
            NumericRepresentation::Double(x) => Ok(x),
            NumericRepresentation::Decimal(bytes) => Ok(str::from_utf8(bytes)?.parse()?),
        }
    }
}

/// The returned pointer is *only* valid for the lifetime to the argument of
/// `from_sql`. This impl is intended for uses where you want to write a new
/// impl in terms of `String`, but don't want to allocate. We have to return a
/// raw pointer instead of a reference with a lifetime due to the structure of
/// `FromSql`
#[cfg(feature = "mysql_backend")]
impl FromSql<Text, Mysql> for *const str {
    fn from_sql(value: MysqlValue<'_>) -> deserialize::Result<Self> {
        let string = str::from_utf8(value.as_bytes())?;
        Ok(string as *const str)
    }
}

#[cfg(feature = "mysql_backend")]
impl<'a> FromSqlRef<'a, Text, Mysql> for &'a str {
    fn from_sql(bytes: &'a mut MysqlValue<'_>) -> deserialize::Result<Self> {
        let string = str::from_utf8(bytes.as_bytes())?;
        Ok(string)
    }
}

#[cfg(feature = "mysql_backend")]
impl Queryable<Text, Mysql> for *const str {
    type Row = Self;

    fn build(row: Self::Row) -> deserialize::Result<Self> {
        Ok(row)
    }
}

/// The returned pointer is *only* valid for the lifetime to the argument of
/// `from_sql`. This impl is intended for uses where you want to write a new
/// impl in terms of `Vec<u8>`, but don't want to allocate. We have to return a
/// raw pointer instead of a reference with a lifetime due to the structure of
/// `FromSql`
#[cfg(feature = "mysql_backend")]
impl FromSql<Binary, Mysql> for *const [u8] {
    fn from_sql(value: MysqlValue<'_>) -> deserialize::Result<Self> {
        Ok(value.as_bytes() as *const [u8])
    }
}

#[cfg(feature = "mysql_backend")]
impl<'a> FromSqlRef<'a, Binary, Mysql> for &'a [u8] {
    fn from_sql(bytes: &'a mut MysqlValue<'_>) -> deserialize::Result<Self> {
        Ok(bytes.as_bytes())
    }
}

#[cfg(feature = "mysql_backend")]
impl Queryable<Binary, Mysql> for *const [u8] {
    type Row = Self;

    fn build(row: Self::Row) -> deserialize::Result<Self> {
        Ok(row)
    }
}
