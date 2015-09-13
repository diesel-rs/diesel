extern crate byteorder;

use self::byteorder::{ReadBytesExt, WriteBytesExt, BigEndian};
use super::option::UnexpectedNullError;
use types::{FromSql, ToSql, IsNull};
use types;
use std::error::Error;
use std::io::Write;

impl FromSql<types::SmallInt> for i16 {
    fn from_sql(bytes: Option<&[u8]>) -> Result<Self, Box<Error>> {
        let mut bytes = not_none!(bytes);
        bytes.read_i16::<BigEndian>().map_err(|e| Box::new(e) as Box<Error>)
    }
}

impl ToSql<types::SmallInt> for i16 {
    fn to_sql<W: Write>(&self, out: &mut W) -> Result<IsNull, Box<Error>> {
        out.write_i16::<BigEndian>(*self)
            .map(|_| IsNull::No)
            .map_err(|e| Box::new(e) as Box<Error>)
    }
}

impl FromSql<types::SmallSerial> for i16 {
    fn from_sql(bytes: Option<&[u8]>) -> Result<Self, Box<Error>> {
        <Self as FromSql<types::SmallInt>>::from_sql(bytes)
    }
}

impl ToSql<types::SmallSerial> for i16 {
    fn to_sql<W: Write>(&self, out: &mut W) -> Result<IsNull, Box<Error>> {
        ToSql::<types::SmallInt>::to_sql(self, out)
    }
}

impl FromSql<types::Integer> for i32 {
    fn from_sql(bytes: Option<&[u8]>) -> Result<Self, Box<Error>> {
        let mut bytes = not_none!(bytes);
        bytes.read_i32::<BigEndian>().map_err(|e| Box::new(e) as Box<Error>)
    }
}

impl ToSql<types::Integer> for i32 {
    fn to_sql<W: Write>(&self, out: &mut W) -> Result<IsNull, Box<Error>> {
        out.write_i32::<BigEndian>(*self)
            .map(|_| IsNull::No)
            .map_err(|e| Box::new(e) as Box<Error>)
    }
}

impl FromSql<types::Serial> for i32 {
    fn from_sql(bytes: Option<&[u8]>) -> Result<Self, Box<Error>> {
        <Self as FromSql<types::Integer>>::from_sql(bytes)
    }
}

impl ToSql<types::Serial> for i32 {
    fn to_sql<W: Write>(&self, out: &mut W) -> Result<IsNull, Box<Error>> {
        ToSql::<types::Integer>::to_sql(self, out)
    }
}

impl FromSql<types::BigInt> for i64 {
    fn from_sql(bytes: Option<&[u8]>) -> Result<Self, Box<Error>> {
        let mut bytes = not_none!(bytes);
        bytes.read_i64::<BigEndian>().map_err(|e| Box::new(e) as Box<Error>)
    }
}

impl ToSql<types::BigInt> for i64 {
    fn to_sql<W: Write>(&self, out: &mut W) -> Result<IsNull, Box<Error>> {
        out.write_i64::<BigEndian>(*self)
            .map(|_| IsNull::No)
            .map_err(|e| Box::new(e) as Box<Error>)
    }
}

impl FromSql<types::BigSerial> for i64 {
    fn from_sql(bytes: Option<&[u8]>) -> Result<Self, Box<Error>> {
        <Self as FromSql<types::BigInt>>::from_sql(bytes)
    }
}

impl ToSql<types::BigSerial> for i64 {
    fn to_sql<W: Write>(&self, out: &mut W) -> Result<IsNull, Box<Error>> {
        ToSql::<types::BigInt>::to_sql(self, out)
    }
}

#[test]
fn i16_to_sql() {
    let mut bytes = vec![];
    ToSql::<types::SmallInt>::to_sql(&1i16, &mut bytes).unwrap();
    ToSql::<types::SmallInt>::to_sql(&0i16, &mut bytes).unwrap();
    ToSql::<types::SmallInt>::to_sql(&-1i16, &mut bytes).unwrap();
    assert_eq!(bytes, vec![0, 1, 0, 0, 255, 255]);
}

#[test]
fn i32_to_sql() {
    let mut bytes = vec![];
    ToSql::<types::Integer>::to_sql(&1i32, &mut bytes).unwrap();
    ToSql::<types::Integer>::to_sql(&0i32, &mut bytes).unwrap();
    ToSql::<types::Integer>::to_sql(&-1i32, &mut bytes).unwrap();
    assert_eq!(bytes, vec![0, 0, 0, 1, 0, 0, 0, 0, 255, 255, 255, 255]);
}

#[test]
fn i64_to_sql() {
    let mut bytes = vec![];
    ToSql::<types::BigInt>::to_sql(&1i64, &mut bytes).unwrap();
    ToSql::<types::BigInt>::to_sql(&0i64, &mut bytes).unwrap();
    ToSql::<types::BigInt>::to_sql(&-1i64, &mut bytes).unwrap();
    assert_eq!(bytes, vec![
               0, 0, 0, 0, 0, 0, 0, 1,
               0, 0, 0, 0, 0, 0, 0, 0,
               255, 255, 255, 255, 255, 255, 255, 255]);
}
