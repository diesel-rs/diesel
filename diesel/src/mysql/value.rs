use super::types::date_and_time::MysqlTime;
use super::MysqlType;
use crate::deserialize;
use std::error::Error;

/// Raw mysql value as received from the database
#[derive(Clone, Debug)]
pub struct MysqlValue<'a> {
    raw: &'a [u8],
    tpe: MysqlType,
}

impl<'a> MysqlValue<'a> {
    /// Create a new instance of [MysqlValue] based on a byte buffer
    /// and information about the type of the value represented by the
    /// given buffer
    #[cfg(feature = "i-implement-a-third-party-backend-and-opt-into-breaking-changes")]
    pub fn new(raw: &'a [u8], tpe: MysqlType) -> Self {
        Self::new_internal(raw, tpe)
    }

    pub(in crate::mysql) fn new_internal(raw: &'a [u8], tpe: MysqlType) -> Self {
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
    /// `MysqlTime` pointer
    // We use `ptr.read_unaligned()` to read the potential unaligned ptr,
    // so clippy is clearly wrong here
    // https://github.com/rust-lang/rust-clippy/issues/2881
    #[allow(dead_code, clippy::cast_ptr_alignment)]
    #[allow(unsafe_code)] // pointer cast
    pub(crate) fn time_value(&self) -> deserialize::Result<MysqlTime> {
        match self.tpe {
            MysqlType::Time | MysqlType::Date | MysqlType::DateTime | MysqlType::Timestamp => {
                self.too_short_buffer(std::mem::size_of::<MysqlTime>(), "Timestamp")?;
                let ptr = self.raw.as_ptr() as *const MysqlTime;
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
    pub(crate) fn numeric_value(&self) -> deserialize::Result<NumericRepresentation<'_>> {
        Ok(match self.tpe {
            MysqlType::UnsignedTiny | MysqlType::Tiny => {
                NumericRepresentation::Tiny(self.raw[0].try_into()?)
            }
            MysqlType::UnsignedShort | MysqlType::Short => {
                self.too_short_buffer(2, "Short")?;
                NumericRepresentation::Small(i16::from_ne_bytes((&self.raw[..2]).try_into()?))
            }
            MysqlType::UnsignedLong | MysqlType::Long => {
                self.too_short_buffer(4, "Long")?;
                NumericRepresentation::Medium(i32::from_ne_bytes((&self.raw[..4]).try_into()?))
            }
            MysqlType::UnsignedLongLong | MysqlType::LongLong => {
                self.too_short_buffer(8, "LongLong")?;
                NumericRepresentation::Big(i64::from_ne_bytes(self.raw.try_into()?))
            }
            MysqlType::Float => {
                self.too_short_buffer(4, "Float")?;
                NumericRepresentation::Float(f32::from_ne_bytes(self.raw.try_into()?))
            }
            MysqlType::Double => {
                self.too_short_buffer(8, "Double")?;
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

    fn too_short_buffer(&self, expected: usize, tpe: &'static str) -> deserialize::Result<()> {
        if self.raw.len() < expected {
            Err(format!(
                "Received a buffer with an invalid size while trying \
             to read a {tpe} value: Expected at least {expected} bytes \
             but got {}",
                self.raw.len()
            )
            .into())
        } else {
            Ok(())
        }
    }
}

/// Represents all possible forms MySQL transmits integers
#[derive(Debug, Clone, Copy)]
#[non_exhaustive]
pub enum NumericRepresentation<'a> {
    /// Corresponds to `MYSQL_TYPE_TINY`
    Tiny(i8),
    /// Corresponds to `MYSQL_TYPE_SHORT`
    Small(i16),
    /// Corresponds to `MYSQL_TYPE_INT24` and `MYSQL_TYPE_LONG`
    Medium(i32),
    /// Corresponds to `MYSQL_TYPE_LONGLONG`
    Big(i64),
    /// Corresponds to `MYSQL_TYPE_FLOAT`
    Float(f32),
    /// Corresponds to `MYSQL_TYPE_DOUBLE`
    Double(f64),
    /// Corresponds to `MYSQL_TYPE_DECIMAL` and `MYSQL_TYPE_NEWDECIMAL`
    Decimal(&'a [u8]),
}

#[test]
fn invalid_reads() {
    assert!(MysqlValue::new_internal(&[1], MysqlType::Timestamp)
        .time_value()
        .is_err());

    assert!(MysqlValue::new_internal(&[1, 2], MysqlType::Long)
        .numeric_value()
        .is_err());

    assert!(MysqlValue::new_internal(&[1, 2, 3, 4], MysqlType::LongLong)
        .numeric_value()
        .is_err());

    assert!(MysqlValue::new_internal(&[1], MysqlType::Short)
        .numeric_value()
        .is_err());

    assert!(MysqlValue::new_internal(&[1, 2, 3, 4], MysqlType::Double)
        .numeric_value()
        .is_err());

    assert!(MysqlValue::new_internal(&[1, 2], MysqlType::Float)
        .numeric_value()
        .is_err());

    assert!(MysqlValue::new_internal(&[1], MysqlType::Tiny)
        .numeric_value()
        .is_ok());
}
