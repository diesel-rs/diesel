use byteorder::{NetworkEndian, ReadBytesExt, WriteBytesExt};
use std::io::prelude::*;

use crate::deserialize::{self, FromSql};
use crate::pg::{Pg, PgValue};
use crate::serialize::{self, IsNull, Output, ToSql};
use crate::sql_types;

impl FromSql<sql_types::Oid, Pg> for u32 {
    fn from_sql(bytes: PgValue<'_>) -> deserialize::Result<Self> {
        let mut bytes = bytes.as_bytes();
        bytes.read_u32::<NetworkEndian>().map_err(Into::into)
    }
}

impl ToSql<sql_types::Oid, Pg> for u32 {
    fn to_sql<W: Write>(&self, out: &mut Output<W, Pg>) -> serialize::Result {
        out.write_u32::<NetworkEndian>(*self)
            .map(|_| IsNull::No)
            .map_err(Into::into)
    }
}

#[test]
fn i16_to_sql() {
    let mut bytes = Output::test();
    ToSql::<sql_types::SmallInt, Pg>::to_sql(&1i16, &mut bytes).unwrap();
    ToSql::<sql_types::SmallInt, Pg>::to_sql(&0i16, &mut bytes).unwrap();
    ToSql::<sql_types::SmallInt, Pg>::to_sql(&-1i16, &mut bytes).unwrap();
    assert_eq!(bytes, vec![0, 1, 0, 0, 255, 255]);
}

#[test]
fn i32_to_sql() {
    let mut bytes = Output::test();
    ToSql::<sql_types::Integer, Pg>::to_sql(&1i32, &mut bytes).unwrap();
    ToSql::<sql_types::Integer, Pg>::to_sql(&0i32, &mut bytes).unwrap();
    ToSql::<sql_types::Integer, Pg>::to_sql(&-1i32, &mut bytes).unwrap();
    assert_eq!(bytes, vec![0, 0, 0, 1, 0, 0, 0, 0, 255, 255, 255, 255]);
}

#[test]
fn i64_to_sql() {
    let mut bytes = Output::test();
    ToSql::<sql_types::BigInt, Pg>::to_sql(&1i64, &mut bytes).unwrap();
    ToSql::<sql_types::BigInt, Pg>::to_sql(&0i64, &mut bytes).unwrap();
    ToSql::<sql_types::BigInt, Pg>::to_sql(&-1i64, &mut bytes).unwrap();
    assert_eq!(
        bytes,
        vec![
            0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 255, 255, 255, 255, 255, 255, 255, 255,
        ]
    );
}
