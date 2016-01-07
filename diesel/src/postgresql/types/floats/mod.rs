extern crate byteorder;

use self::byteorder::{ReadBytesExt, WriteBytesExt, BigEndian};
use super::option::UnexpectedNullError;
use types::{FromSql, ToSql, IsNull};
use types;
use std::error::Error;
use std::io::Write;

#[cfg(feature = "quickcheck")]
mod quickcheck_impls;

impl FromSql<types::Float> for f32 {
    fn from_sql(bytes: Option<&[u8]>) -> Result<Self, Box<Error>> {
        let mut bytes = not_none!(bytes);
        bytes.read_f32::<BigEndian>().map_err(|e| Box::new(e) as Box<Error>)
    }
}

impl ToSql<types::Float> for f32 {
    fn to_sql<W: Write>(&self, out: &mut W) -> Result<IsNull, Box<Error>> {
        out.write_f32::<BigEndian>(*self)
            .map(|_| IsNull::No)
            .map_err(|e| Box::new(e) as Box<Error>)
    }
}

impl FromSql<types::Double> for f64 {
    fn from_sql(bytes: Option<&[u8]>) -> Result<Self, Box<Error>> {
        let mut bytes = not_none!(bytes);
        bytes.read_f64::<BigEndian>().map_err(|e| Box::new(e) as Box<Error>)
    }
}

impl ToSql<types::Double> for f64 {
    fn to_sql<W: Write>(&self, out: &mut W) -> Result<IsNull, Box<Error>> {
        out.write_f64::<BigEndian>(*self)
            .map(|_| IsNull::No)
            .map_err(|e| Box::new(e) as Box<Error>)
    }
}

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

impl FromSql<types::Numeric> for PgNumeric {
    fn from_sql(bytes: Option<&[u8]>) -> Result<Self, Box<Error>> {
        let mut bytes = not_none!(bytes);
        let ndigits = try!(bytes.read_u16::<BigEndian>());
        let mut digits = Vec::with_capacity(ndigits as usize);
        let weight = try!(bytes.read_i16::<BigEndian>());
        let sign = try!(bytes.read_u16::<BigEndian>());
        let scale = try!(bytes.read_u16::<BigEndian>());
        for _ in 0..ndigits {
            digits.push(try!(bytes.read_i16::<BigEndian>()));
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

impl ToSql<types::Numeric> for PgNumeric {
    fn to_sql<W: Write>(&self, out: &mut W) -> Result<IsNull, Box<Error>> {
        let sign = match self {
            &PgNumeric::Positive { .. } => 0,
            &PgNumeric::Negative { .. } => 0x4000,
            &PgNumeric::NaN => 0xC000,
        };
        let empty_vec = Vec::new();
        let digits = match self {
            &PgNumeric::Positive { ref digits, .. } => digits,
            &PgNumeric::Negative { ref digits, .. } => digits,
            &PgNumeric::NaN => &empty_vec,
        };
        let weight = match self {
            &PgNumeric::Positive { weight, .. } => weight,
            &PgNumeric::Negative { weight, .. } => weight,
            &PgNumeric::NaN => 0,
        };
        let scale = match self {
            &PgNumeric::Positive { scale, .. } => scale,
            &PgNumeric::Negative { scale, .. } => scale,
            &PgNumeric::NaN => 0,
        };
        try!(out.write_u16::<BigEndian>(digits.len() as u16));
        try!(out.write_i16::<BigEndian>(weight));
        try!(out.write_u16::<BigEndian>(sign));
        try!(out.write_u16::<BigEndian>(scale));
        for digit in digits.iter() {
            try!(out.write_i16::<BigEndian>(*digit));
        }

        Ok(IsNull::No)
    }
}
