use byteorder::{ReadBytesExt, WriteBytesExt, NetworkEndian};
use std::error::Error;
use std::io::prelude::*;

use pg::Pg;
use types::{self, ToSql, ToSqlOutput, IsNull, FromSql};

primitive_impls!(Oid -> (u32, pg: (26, 1018)));

impl FromSql<types::Oid, Pg> for u32 {
    fn from_sql(bytes: Option<&[u8]>) -> Result<Self, Box<Error+Send+Sync>> {
        let mut bytes = not_none!(bytes);
        bytes.read_u32::<NetworkEndian>().map_err(|e| e.into())
    }
}

impl ToSql<types::Oid, Pg> for u32 {
    fn to_sql<W: Write>(&self, out: &mut ToSqlOutput<W, Pg>) -> Result<IsNull, Box<Error+Send+Sync>> {
        out.write_u32::<NetworkEndian>(*self)
            .map(|_| IsNull::No)
            .map_err(|e| e.into())
    }
}

#[test]
fn i16_to_sql() {
    let mut bytes = vec![];
    ToSql::<types::SmallInt, Pg>::to_sql(&1i16, &mut bytes).unwrap();
    ToSql::<types::SmallInt, Pg>::to_sql(&0i16, &mut bytes).unwrap();
    ToSql::<types::SmallInt, Pg>::to_sql(&-1i16, &mut bytes).unwrap();
    assert_eq!(bytes, vec![0, 1, 0, 0, 255, 255]);
}

#[test]
fn i32_to_sql() {
    let mut bytes = vec![];
    ToSql::<types::Integer, Pg>::to_sql(&1i32, &mut bytes).unwrap();
    ToSql::<types::Integer, Pg>::to_sql(&0i32, &mut bytes).unwrap();
    ToSql::<types::Integer, Pg>::to_sql(&-1i32, &mut bytes).unwrap();
    assert_eq!(bytes, vec![0, 0, 0, 1, 0, 0, 0, 0, 255, 255, 255, 255]);
}

#[test]
fn i64_to_sql() {
    let mut bytes = vec![];
    ToSql::<types::BigInt, Pg>::to_sql(&1i64, &mut bytes).unwrap();
    ToSql::<types::BigInt, Pg>::to_sql(&0i64, &mut bytes).unwrap();
    ToSql::<types::BigInt, Pg>::to_sql(&-1i64, &mut bytes).unwrap();
    assert_eq!(bytes, vec![
               0, 0, 0, 0, 0, 0, 0, 1,
               0, 0, 0, 0, 0, 0, 0, 0,
               255, 255, 255, 255, 255, 255, 255, 255]);
}
