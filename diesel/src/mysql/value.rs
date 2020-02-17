use super::{MysqlType, MysqlTypeMetadata};
use crate::deserialize;
use mysqlclient_sys as ffi;
use std::error::Error;

/// Raw mysql value as received from the database
#[derive(Copy, Clone, Debug)]
pub struct MysqlValue<'a> {
    raw: &'a [u8],
    tpe: MysqlTypeMetadata,
}

impl<'a> MysqlValue<'a> {
    pub(crate) fn new(raw: &'a [u8], tpe: MysqlTypeMetadata) -> Self {
        Self { raw, tpe }
    }

    /// Get the underlying raw byte representation
    pub fn as_bytes(&self) -> &[u8] {
        self.raw
    }

    /// Checks that the type code is valid, and interprets the data as a
    /// `MYSQL_TIME` pointer
    #[allow(dead_code)]
    pub(crate) fn time_value(&self) -> deserialize::Result<ffi::MYSQL_TIME> {
        match self.tpe.data_type {
            MysqlType::Time | MysqlType::Date | MysqlType::DateTime | MysqlType::Timestamp => {
                Ok(*unsafe { &*(self.raw as *const _ as *const ffi::MYSQL_TIME) })
            }
            _ => Err(self.invalid_type_code("timestamp")),
        }
    }

    /// Returns the numeric representation of this value, based on the type code.
    /// Returns an error if the type code is not numeric.
    pub(crate) fn numeric_value(&self) -> deserialize::Result<NumericRepresentation> {
        use self::NumericRepresentation::*;
        use std::convert::TryInto;

        Ok(match self.tpe.data_type {
            MysqlType::Tiny => Tiny(self.raw[0] as i8),
            MysqlType::Short => Small(i16::from_ne_bytes(self.raw.try_into()?)),
            MysqlType::Long => Medium(i32::from_ne_bytes(self.raw.try_into()?)),
            MysqlType::LongLong => Big(i64::from_ne_bytes(self.raw.try_into()?)),
            MysqlType::Float => Float(f32::from_ne_bytes(self.raw.try_into()?)),
            MysqlType::Double => Double(f64::from_ne_bytes(self.raw.try_into()?)),

            MysqlType::Numeric => Decimal(self.raw),
            _ => return Err(self.invalid_type_code("number")),
        })
    }

    fn invalid_type_code(&self, expected: &str) -> Box<dyn Error + Send + Sync> {
        format!(
            "Invalid representation received for {}: {:?}",
            expected, self.tpe
        )
        .into()
    }
}

/// Represents all possible forms MySQL transmits integers
#[derive(Debug, Clone, Copy)]
pub enum NumericRepresentation<'a> {
    /// Correponds to `MYSQL_TYPE_TINY`
    Tiny(i8),
    /// Correponds to `MYSQL_TYPE_SHORT`
    Small(i16),
    /// Correponds to `MYSQL_TYPE_INT24` and `MYSQL_TYPE_LONG`
    Medium(i32),
    /// Correponds to `MYSQL_TYPE_LONGLONG`
    Big(i64),
    /// Correponds to `MYSQL_TYPE_FLOAT`
    Float(f32),
    /// Correponds to `MYSQL_TYPE_DOUBLE`
    Double(f64),
    /// Correponds to `MYSQL_TYPE_DECIMAL` and `MYSQL_TYPE_NEWDECIMAL`
    Decimal(&'a [u8]),
}
