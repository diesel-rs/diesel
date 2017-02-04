use byteorder::{ReadBytesExt, WriteBytesExt, NetworkEndian};
use std::error::Error;
use std::io::prelude::*;

use pg::Pg;
use types::{self, IsNull, FromSql, ToSql};

#[cfg(feature = "quickcheck")]
mod quickcheck_impls;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PgNumeric {
    Positive {
        weight: i16,
        scale: u16,
        digits: Vec<i16>,
    },
    Negative {
        weight: i16,
        scale: u16,
        digits: Vec<i16>,
    },
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

impl FromSql<types::Numeric, Pg> for PgNumeric {
    fn from_sql(bytes: Option<&[u8]>) -> Result<Self, Box<Error+Send+Sync>> {
        let mut bytes = not_none!(bytes);
        let num_digits = try!(bytes.read_u16::<NetworkEndian>());
        let mut digits = Vec::with_capacity(num_digits as usize);
        let weight = try!(bytes.read_i16::<NetworkEndian>());
        let sign = try!(bytes.read_u16::<NetworkEndian>());
        let scale = try!(bytes.read_u16::<NetworkEndian>());
        for _ in 0..num_digits {
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

impl ToSql<types::Numeric, Pg> for PgNumeric {
    fn to_sql<W: Write>(&self, out: &mut W) -> Result<IsNull, Box<Error+Send+Sync>> {
        let sign = match *self {
            PgNumeric::Positive { .. } => 0,
            PgNumeric::Negative { .. } => 0x4000,
            PgNumeric::NaN => 0xC000,
        };
        let empty_vec = Vec::new();
        let digits = match *self {
            PgNumeric::Positive { ref digits, .. } |
            PgNumeric::Negative { ref digits, .. } => digits,
            PgNumeric::NaN => &empty_vec,
        };
        let weight = match *self {
            PgNumeric::Positive { weight, .. } |
            PgNumeric::Negative { weight, .. } => weight,
            PgNumeric::NaN => 0,
        };
        let scale = match *self {
            PgNumeric::Positive { scale, .. } |
            PgNumeric::Negative { scale, .. } => scale,
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
