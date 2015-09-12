extern crate byteorder;

use self::byteorder::{ReadBytesExt, WriteBytesExt, BigEndian};
use super::option::UnexpectedNullError;
use types::{FromSql, ToSql};
use types;
use std::error::Error;
use std::io::Write;

impl FromSql<types::Float> for f32 {
    fn from_sql(bytes: Option<&[u8]>) -> Result<Self, Box<Error>> {
        let mut bytes = not_none!(bytes);
        bytes.read_f32::<BigEndian>().map_err(|e| Box::new(e) as Box<Error>)
    }
}

impl ToSql<types::Float> for f32 {
    fn to_sql<W: Write>(&self, out: &mut W) -> Result<(), Box<Error>> {
        out.write_f32::<BigEndian>(*self).map_err(|e| Box::new(e) as Box<Error>)
    }
}

impl FromSql<types::Double> for f64 {
    fn from_sql(bytes: Option<&[u8]>) -> Result<Self, Box<Error>> {
        let mut bytes = not_none!(bytes);
        bytes.read_f64::<BigEndian>().map_err(|e| Box::new(e) as Box<Error>)
    }
}

impl ToSql<types::Double> for f64 {
    fn to_sql<W: Write>(&self, out: &mut W) -> Result<(), Box<Error>> {
        out.write_f64::<BigEndian>(*self).map_err(|e| Box::new(e) as Box<Error>)
    }
}
