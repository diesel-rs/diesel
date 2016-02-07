extern crate byteorder;

use std::error::Error;
use std::io::prelude::*;
use self::byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};

use backend::Backend;
use types::{self, FromSql, IsNull, ToSql};

impl<DB: Backend<RawValue = [u8]>> FromSql<types::SmallInt, DB> for i16 {
    fn from_sql(bytes: Option<&[u8]>) -> Result<Self, Box<Error>> {
        let mut bytes = not_none!(bytes);
        bytes.read_i16::<BigEndian>().map_err(|e| Box::new(e) as Box<Error>)
    }
}

impl<DB: Backend> ToSql<types::SmallInt, DB> for i16 {
    fn to_sql<W: Write>(&self, out: &mut W) -> Result<IsNull, Box<Error>> {
        out.write_i16::<BigEndian>(*self)
           .map(|_| IsNull::No)
           .map_err(|e| Box::new(e) as Box<Error>)
    }
}

impl<DB: Backend<RawValue = [u8]>> FromSql<types::Integer, DB> for i32 {
    fn from_sql(bytes: Option<&[u8]>) -> Result<Self, Box<Error>> {
        let mut bytes = not_none!(bytes);
        bytes.read_i32::<BigEndian>().map_err(|e| Box::new(e) as Box<Error>)
    }
}

impl<DB: Backend> ToSql<types::Integer, DB> for i32 {
    fn to_sql<W: Write>(&self, out: &mut W) -> Result<IsNull, Box<Error>> {
        out.write_i32::<BigEndian>(*self)
           .map(|_| IsNull::No)
           .map_err(|e| Box::new(e) as Box<Error>)
    }
}

impl<DB: Backend<RawValue = [u8]>> FromSql<types::BigInt, DB> for i64 {
    fn from_sql(bytes: Option<&[u8]>) -> Result<Self, Box<Error>> {
        let mut bytes = not_none!(bytes);
        bytes.read_i64::<BigEndian>().map_err(|e| Box::new(e) as Box<Error>)
    }
}

impl<DB: Backend> ToSql<types::BigInt, DB> for i64 {
    fn to_sql<W: Write>(&self, out: &mut W) -> Result<IsNull, Box<Error>> {
        out.write_i64::<BigEndian>(*self)
           .map(|_| IsNull::No)
           .map_err(|e| Box::new(e) as Box<Error>)
    }
}
