use crate::deserialize::{self, FromSql, FromSqlRow};
use crate::expression::AsExpression;
use crate::pg::{Pg, PgValue};
use crate::serialize::{self, IsNull, Output, ToSql};
use crate::sql_types;
use byteorder::{NetworkEndian, ReadBytesExt, WriteBytesExt};
use std::error::Error;

#[cfg(feature = "quickcheck")]
mod quickcheck_impls;

#[derive(Debug, Clone, PartialEq, Eq, AsExpression, FromSqlRow)]
#[diesel(sql_type = sql_types::Numeric)]
/// Represents a NUMERIC value, closely mirroring the PG wire protocol
/// representation
pub enum PgNumeric {
    /// A positive number
    Positive {
        /// How many digits come before the decimal point?
        weight: i16,
        /// How many significant digits are there?
        scale: u16,
        /// The digits in this number, stored in base 10000
        digits: Vec<i16>,
    },
    /// A negative number
    Negative {
        /// How many digits come before the decimal point?
        weight: i16,
        /// How many significant digits are there?
        scale: u16,
        /// The digits in this number, stored in base 10000
        digits: Vec<i16>,
    },
    /// Not a number
    NaN,
}

#[derive(Debug, Clone, Copy)]
#[allow(dead_code)] // that's used by debug in the error impl
struct InvalidNumericSign(u16);

impl ::std::fmt::Display for InvalidNumericSign {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> ::std::fmt::Result {
        f.write_str("sign for numeric field was not one of 0, 0x4000, 0xC000")
    }
}

impl Error for InvalidNumericSign {}

#[cfg(feature = "postgres_backend")]
impl FromSql<sql_types::Numeric, Pg> for PgNumeric {
    fn from_sql(bytes: PgValue<'_>) -> deserialize::Result<Self> {
        let mut bytes = bytes.as_bytes();
        let digit_count = bytes.read_u16::<NetworkEndian>()?;
        let mut digits = Vec::with_capacity(digit_count as usize);
        let weight = bytes.read_i16::<NetworkEndian>()?;
        let sign = bytes.read_u16::<NetworkEndian>()?;
        let scale = bytes.read_u16::<NetworkEndian>()?;
        for _ in 0..digit_count {
            digits.push(bytes.read_i16::<NetworkEndian>()?);
        }

        match sign {
            0 => Ok(PgNumeric::Positive {
                weight,
                scale,
                digits,
            }),
            0x4000 => Ok(PgNumeric::Negative {
                weight,
                scale,
                digits,
            }),
            0xC000 => Ok(PgNumeric::NaN),
            invalid => Err(Box::new(InvalidNumericSign(invalid))),
        }
    }
}

#[cfg(feature = "postgres_backend")]
impl ToSql<sql_types::Numeric, Pg> for PgNumeric {
    fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, Pg>) -> serialize::Result {
        let sign = match *self {
            PgNumeric::Positive { .. } => 0,
            PgNumeric::Negative { .. } => 0x4000,
            PgNumeric::NaN => 0xC000,
        };
        let empty_vec = Vec::new();
        let digits = match *self {
            PgNumeric::Positive { ref digits, .. } | PgNumeric::Negative { ref digits, .. } => {
                digits
            }
            PgNumeric::NaN => &empty_vec,
        };
        let weight = match *self {
            PgNumeric::Positive { weight, .. } | PgNumeric::Negative { weight, .. } => weight,
            PgNumeric::NaN => 0,
        };
        let scale = match *self {
            PgNumeric::Positive { scale, .. } | PgNumeric::Negative { scale, .. } => scale,
            PgNumeric::NaN => 0,
        };
        out.write_u16::<NetworkEndian>(digits.len() as u16)?;
        out.write_i16::<NetworkEndian>(weight)?;
        out.write_u16::<NetworkEndian>(sign)?;
        out.write_u16::<NetworkEndian>(scale)?;
        for digit in digits.iter() {
            out.write_i16::<NetworkEndian>(*digit)?;
        }

        Ok(IsNull::No)
    }
}

#[cfg(feature = "postgres_backend")]
impl FromSql<sql_types::Float, Pg> for f32 {
    fn from_sql(value: PgValue<'_>) -> deserialize::Result<Self> {
        let mut bytes = value.as_bytes();

        if bytes.len() < 4 {
            return deserialize::Result::Err(
                "Received less than 4 bytes while decoding an f32. \
                 Was a numeric accidentally marked as float?"
                    .into(),
            );
        }

        if bytes.len() > 4 {
            return deserialize::Result::Err(
                "Received more than 4 bytes while decoding an f32. \
                 Was a double accidentally marked as float?"
                    .into(),
            );
        }

        bytes
            .read_f32::<NetworkEndian>()
            .map_err(|e| Box::new(e) as Box<dyn Error + Send + Sync>)
    }
}

#[cfg(feature = "postgres_backend")]
impl FromSql<sql_types::Double, Pg> for f64 {
    fn from_sql(value: PgValue<'_>) -> deserialize::Result<Self> {
        let mut bytes = value.as_bytes();

        if bytes.len() < 8 {
            return deserialize::Result::Err(
                "Received less than 8 bytes while decoding an f64. \
                    Was a float accidentally marked as double?"
                    .into(),
            );
        }

        if bytes.len() > 8 {
            return deserialize::Result::Err(
                "Received more than 8 bytes while decoding an f64. \
                    Was a numeric accidentally marked as double?"
                    .into(),
            );
        }

        bytes
            .read_f64::<NetworkEndian>()
            .map_err(|e| Box::new(e) as Box<dyn Error + Send + Sync>)
    }
}

#[cfg(feature = "postgres_backend")]
impl ToSql<sql_types::Float, Pg> for f32 {
    fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, Pg>) -> serialize::Result {
        out.write_f32::<NetworkEndian>(*self)
            .map(|_| IsNull::No)
            .map_err(|e| Box::new(e) as Box<dyn Error + Send + Sync>)
    }
}

#[cfg(feature = "postgres_backend")]
impl ToSql<sql_types::Double, Pg> for f64 {
    fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, Pg>) -> serialize::Result {
        out.write_f64::<NetworkEndian>(*self)
            .map(|_| IsNull::No)
            .map_err(|e| Box::new(e) as Box<dyn Error + Send + Sync>)
    }
}
