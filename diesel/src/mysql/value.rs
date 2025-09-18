use super::types::date_and_time::MysqlTime;
use super::MysqlType;

use crate::deserialize;
use std::error::Error;
use std::mem::MaybeUninit;

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
    // We use `ptr::copy` to read the actual data
    // and copy it over to the returned `MysqlTime` instance
    #[allow(unsafe_code)] // MaybeUninit + ptr copy
    pub(crate) fn time_value(&self) -> deserialize::Result<MysqlTime> {
        match self.tpe {
            MysqlType::Time | MysqlType::Date | MysqlType::DateTime | MysqlType::Timestamp => {
                // we check for the size of the `MYSQL_TIME` type from `mysqlclient_sys` here as
                // certain older libmysqlclient and newer libmariadb versions do not have all the
                // same fields (and size) as the `MysqlTime` type from diesel. The later one is modeled after
                // the type from newer libmysqlclient
                self.too_short_buffer(
                    #[cfg(feature = "mysql")]
                    std::mem::size_of::<mysqlclient_sys::MYSQL_TIME>(),
                    #[cfg(not(feature = "mysql"))]
                    std::mem::size_of::<MysqlTime>(),
                    "timestamp",
                )?;
                // To ensure we copy the right number of bytes we need to make sure to copy not more bytes than needed
                // for `MysqlTime` and not more bytes than inside of the buffer
                let len = std::cmp::min(std::mem::size_of::<MysqlTime>(), self.raw.len());
                // Zero is a valid pattern for this type so we are fine with initializing all fields to zero
                // If the provided byte buffer is too short we just use 0 as default value
                let mut out = MaybeUninit::<MysqlTime>::zeroed();
                // Make sure to check that the boolean is an actual bool value, so 0 or 1
                // as anything else is UB in rust
                let neg_offset = std::mem::offset_of!(MysqlTime, neg);
                if neg_offset < self.raw.len()
                    && self.raw[neg_offset] != 0
                    && self.raw[neg_offset] != 1
                {
                    return Err(
                        "Received invalid value for `neg` in the `MysqlTime` datastructure".into(),
                    );
                }
                let result = unsafe {
                    // SAFETY: We copy over the bytes from our raw buffer to the `MysqlTime` instance
                    // This type is correctly aligned and we ensure that we do not copy more bytes than are there
                    // We are also sure that these ptr do not overlap as they are completely different
                    // instances
                    std::ptr::copy_nonoverlapping(
                        self.raw.as_ptr(),
                        out.as_mut_ptr() as *mut u8,
                        len,
                    );
                    // SAFETY: all zero is a valid pattern for this type
                    // Otherwise any other bit pattern is also valid, beside
                    // neg being something other than 0 or 1
                    // We check for that above by looking at the byte before copying
                    out.assume_init()
                };
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
#[allow(unsafe_code, reason = "Test code")]
fn invalid_reads() {
    use crate::data_types::MysqlTimestampType;

    assert!(MysqlValue::new_internal(&[1], MysqlType::Timestamp)
        .time_value()
        .is_err());
    let v = MysqlTime {
        year: 2025,
        month: 9,
        day: 15,
        hour: 22,
        minute: 3,
        second: 10,
        second_part: 0,
        neg: false,
        time_type: MysqlTimestampType::MYSQL_TIMESTAMP_DATETIME,
        time_zone_displacement: 0,
    };
    let mut bytes = [0; std::mem::size_of::<MysqlTime>()];
    unsafe {
        // SAFETY: Test code
        // also the size matches and we want to get raw bytes
        std::ptr::copy(
            &v as *const MysqlTime as *const u8,
            bytes.as_mut_ptr(),
            bytes.len(),
        );
    }
    let offset = std::mem::offset_of!(MysqlTime, neg);
    bytes[offset] = 42;
    assert!(MysqlValue::new_internal(&bytes, MysqlType::Timestamp)
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
