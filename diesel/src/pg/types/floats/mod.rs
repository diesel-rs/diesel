use byteorder::{NetworkEndian, ReadBytesExt, WriteBytesExt};
use std::error::Error;
use std::io::prelude::*;

use deserialize::{self, FromSql};
use pg::Pg;
use serialize::{self, IsNull, Output, ToSql};
use sql_types;

#[cfg(feature = "quickcheck")]
mod quickcheck_impls;

#[derive(Debug, Clone, PartialEq, Eq, FromSqlRow, AsExpression)]
#[sql_type = "sql_types::Numeric"]
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
struct InvalidNumericSign(u16);

impl ::std::fmt::Display for InvalidNumericSign {
    fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
        write!(f, "InvalidNumericSign({0:x})", self.0)
    }
}

impl Error for InvalidNumericSign {
    fn description(&self) -> &str {
        "sign for numeric field was not one of 0, 0x4000, 0xC000"
    }
}

impl FromSql<sql_types::Numeric, Pg> for PgNumeric {
    fn from_sql(bytes: Option<&[u8]>) -> deserialize::Result<Self> {
        let mut bytes = not_none!(bytes);
        let digit_count = try!(bytes.read_u16::<NetworkEndian>());
        let mut digits = Vec::with_capacity(digit_count as usize);
        let weight = try!(bytes.read_i16::<NetworkEndian>());
        let sign = try!(bytes.read_u16::<NetworkEndian>());
        let scale = try!(bytes.read_u16::<NetworkEndian>());
        for _ in 0..digit_count {
            digits.push(try!(bytes.read_i16::<NetworkEndian>()));
        }

        match sign {
            0 => Ok(PgNumeric::Positive {
                weight: weight,
                scale: scale,
                digits: digits,
            }),
            0x4000 => Ok(PgNumeric::Negative {
                weight: weight,
                scale: scale,
                digits: digits,
            }),
            0xC000 => Ok(PgNumeric::NaN),
            invalid => Err(Box::new(InvalidNumericSign(invalid))),
        }
    }
}

impl ToSql<sql_types::Numeric, Pg> for PgNumeric {
    fn to_sql<W: Write>(&self, out: &mut Output<W, Pg>) -> serialize::Result {
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
        try!(out.write_u16::<NetworkEndian>(digits.len() as u16));
        try!(out.write_i16::<NetworkEndian>(weight));
        try!(out.write_u16::<NetworkEndian>(sign));
        try!(out.write_u16::<NetworkEndian>(scale));
        for digit in digits.iter() {
            try!(out.write_i16::<NetworkEndian>(*digit));
        }

        Ok(IsNull::No)
    }
}
