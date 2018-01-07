use std::str;
use std::error::Error;

use mysql::{Mysql, MysqlValue};
use types::{self, FromSql};

impl FromSql<types::SmallInt, Mysql> for i16 {
    fn from_sql(value: Option<&MysqlValue>) -> Result<Self, Box<Error + Send + Sync>> {
        use mysql::NumericRepresentation::*;

        let data = not_none!(value).numeric_value()?;
        match data {
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

impl FromSql<types::Integer, Mysql> for i32 {
    fn from_sql(value: Option<&MysqlValue>) -> Result<Self, Box<Error + Send + Sync>> {
        use mysql::NumericRepresentation::*;

        let data = not_none!(value).numeric_value()?;
        match data {
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

impl FromSql<types::BigInt, Mysql> for i64 {
    fn from_sql(value: Option<&MysqlValue>) -> Result<Self, Box<Error + Send + Sync>> {
        use mysql::NumericRepresentation::*;

        let data = not_none!(value).numeric_value()?;
        match data {
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

impl FromSql<types::Float, Mysql> for f32 {
    fn from_sql(value: Option<&MysqlValue>) -> Result<Self, Box<Error + Send + Sync>> {
        use mysql::NumericRepresentation::*;

        let data = not_none!(value).numeric_value()?;
        match data {
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

impl FromSql<types::Double, Mysql> for f64 {
    fn from_sql(value: Option<&MysqlValue>) -> Result<Self, Box<Error + Send + Sync>> {
        use mysql::NumericRepresentation::*;

        let data = not_none!(value).numeric_value()?;
        match data {
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

impl FromSql<types::Text, Mysql> for String {
    fn from_sql(value: Option<&MysqlValue>) -> Result<Self, Box<Error + Send + Sync>> {
        let bytes = not_none!(value).bytes()?;
        String::from_utf8(bytes.into()).map_err(Into::into)
    }
}

impl FromSql<types::Binary, Mysql> for Vec<u8> {
    fn from_sql(value: Option<&MysqlValue>) -> Result<Self, Box<Error + Send + Sync>> {
        not_none!(value).bytes().map(Into::into)
    }
}
