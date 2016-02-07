extern crate byteorder;

use std::error::Error;
use std::io::prelude::*;

use backend::Backend;
use self::byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};
use types::{self, FromSql, IsNull, ToSql};

impl<DB: Backend<RawValue = [u8]>> FromSql<types::Float, DB> for f32 {
    fn from_sql(bytes: Option<&[u8]>) -> Result<Self, Box<Error>> {
        let mut bytes = not_none!(bytes);
        bytes.read_f32::<BigEndian>().map_err(|e| Box::new(e) as Box<Error>)
    }
}

impl<DB: Backend> ToSql<types::Float, DB> for f32 {
    fn to_sql<W: Write>(&self, out: &mut W) -> Result<IsNull, Box<Error>> {
        out.write_f32::<BigEndian>(*self)
           .map(|_| IsNull::No)
           .map_err(|e| Box::new(e) as Box<Error>)
    }
}

impl<DB: Backend<RawValue = [u8]>> FromSql<types::Double, DB> for f64 {
    fn from_sql(bytes: Option<&[u8]>) -> Result<Self, Box<Error>> {
        let mut bytes = not_none!(bytes);
        bytes.read_f64::<BigEndian>().map_err(|e| Box::new(e) as Box<Error>)
    }
}

impl<DB: Backend> ToSql<types::Double, DB> for f64 {
    fn to_sql<W: Write>(&self, out: &mut W) -> Result<IsNull, Box<Error>> {
        out.write_f64::<BigEndian>(*self)
           .map(|_| IsNull::No)
           .map_err(|e| Box::new(e) as Box<Error>)
    }
}
