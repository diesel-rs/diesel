use super::MysqlType;
use crate::deserialize;
use crate::mysql::types::MYSQL_TIME;
use std::error::Error;

/// Raw mysql value as received from the database
#[derive(Copy, Clone, Debug)]
pub struct MysqlValue<'a> {
    raw: &'a [u8],
    tpe: MysqlType,
}

impl<'a> MysqlValue<'a> {
    pub(crate) fn new(raw: &'a [u8], tpe: MysqlType) -> Self {
        Self { raw, tpe }
    }

    /// Get the underlying raw byte representation
    pub fn as_bytes(&self) -> &[u8] {
        self.raw
    }

    /// Get the mysql type of the current value
    pub fn value_type(&self) -> MysqlType {
        self.tpe
    }

    /// Checks that the type code is valid, and interprets the data as a
    /// `MYSQL_TIME` pointer
    // We use `ptr.read_unaligned()` to read the potential unaligned ptr,
    // so clippy is clearly wrong here
    // https://github.com/rust-lang/rust-clippy/issues/2881
    #[allow(dead_code, clippy::cast_ptr_alignment)]
    pub(crate) fn time_value(&self) -> deserialize::Result<MYSQL_TIME> {
        match self.tpe {
            MysqlType::Time | MysqlType::Date | MysqlType::DateTime | MysqlType::Timestamp => {
                let ptr = self.raw.as_ptr() as *const MYSQL_TIME;
                let result = unsafe { ptr.read_unaligned() };
                if result.neg {
                    Err("Negative dates/times are not yet supported".into())
                } else {
                    Ok(result)
                }
            }
            _ => Err(self.invalid_type_code("timestamp")),
        }
    }

    /// Returns the numeric representation of this value, based on the type code.
    /// Returns an error if the type code is not numeric.
    pub(crate) fn numeric_value(&self) -> deserialize::Result<NumericRepresentation> {
        use std::convert::TryInto;

        Ok(match self.tpe {
            MysqlType::UnsignedTiny | MysqlType::Tiny => {
                NumericRepresentation::Tiny(self.raw[0] as i8)
            }
            MysqlType::UnsignedShort | MysqlType::Short => {
                NumericRepresentation::Small(i16::from_ne_bytes(self.raw.try_into()?))
            }
            MysqlType::UnsignedLong | MysqlType::Long => {
                NumericRepresentation::Medium(i32::from_ne_bytes(self.raw.try_into()?))
            }
            MysqlType::UnsignedLongLong | MysqlType::LongLong => {
                NumericRepresentation::Big(i64::from_ne_bytes(self.raw.try_into()?))
            }
            MysqlType::Float => {
                NumericRepresentation::Float(f32::from_ne_bytes(self.raw.try_into()?))
            }
            MysqlType::Double => {
                NumericRepresentation::Double(f64::from_ne_bytes(self.raw.try_into()?))
            }

            MysqlType::Numeric => NumericRepresentation::Decimal(self.raw),
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
#[non_exhaustive]
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
