use crate::deserialize::{self, FromSql};
use crate::pg::{Pg, PgValue};
use crate::serialize::{self, IsNull, Output, ToSql};
use crate::sql_types;
use byteorder::{NetworkEndian, ReadBytesExt, WriteBytesExt};

impl FromSql<sql_types::Oid, Pg> for u32 {
    fn from_sql(bytes: PgValue<'_>) -> deserialize::Result<Self> {
        let mut bytes = bytes.as_bytes();
        bytes.read_u32::<NetworkEndian>().map_err(Into::into)
    }
}

impl ToSql<sql_types::Oid, Pg> for u32 {
    fn to_sql<'a: 'b, 'b>(&'a self, out: &mut Output<'b, '_, Pg>) -> serialize::Result {
        out.write_u32::<NetworkEndian>(*self)
            .map(|_| IsNull::No)
            .map_err(Into::into)
    }
}

impl FromSql<sql_types::SmallInt, Pg> for i16 {
    fn from_sql(value: PgValue<'_>) -> deserialize::Result<Self> {
        let mut bytes = value.as_bytes();
        debug_assert!(
            bytes.len() <= 2,
            "Received more than 2 bytes decoding i16. \
             Was an Integer expression accidentally identified as SmallInt?"
        );
        debug_assert!(
            bytes.len() >= 2,
            "Received fewer than 2 bytes decoding i16. \
             Was an expression of a different type accidentally identified \
             as SmallInt?"
        );
        bytes
            .read_i16::<NetworkEndian>()
            .map_err(|e| Box::new(e) as Box<_>)
    }
}

impl FromSql<sql_types::Integer, Pg> for i32 {
    fn from_sql(value: PgValue<'_>) -> deserialize::Result<Self> {
        let mut bytes = value.as_bytes();
        debug_assert!(
            bytes.len() <= 4,
            "Received more than 4 bytes decoding i32. \
             Was a BigInt expression accidentally identified as Integer?"
        );
        debug_assert!(
            bytes.len() >= 4,
            "Received fewer than 4 bytes decoding i32. \
             Was a SmallInt expression accidentally identified as Integer?"
        );
        bytes
            .read_i32::<NetworkEndian>()
            .map_err(|e| Box::new(e) as Box<_>)
    }
}

impl FromSql<sql_types::BigInt, Pg> for i64 {
    fn from_sql(value: PgValue<'_>) -> deserialize::Result<Self> {
        let mut bytes = value.as_bytes();
        debug_assert!(
            bytes.len() <= 8,
            "Received more than 8 bytes decoding i64. \
             Was an expression of a different type misidentified as BigInt?"
        );
        debug_assert!(
            bytes.len() >= 8,
            "Received fewer than 8 bytes decoding i64. \
             Was an Integer expression misidentified as BigInt?"
        );
        bytes
            .read_i64::<NetworkEndian>()
            .map_err(|e| Box::new(e) as Box<_>)
    }
}

impl ToSql<sql_types::SmallInt, Pg> for i16 {
    fn to_sql<'a: 'b, 'b>(&'a self, out: &mut Output<'b, '_, Pg>) -> serialize::Result {
        out.write_i16::<NetworkEndian>(*self)
            .map(|_| IsNull::No)
            .map_err(|e| Box::new(e) as Box<_>)
    }
}

impl ToSql<sql_types::Integer, Pg> for i32 {
    fn to_sql<'a: 'b, 'b>(&'a self, out: &mut Output<'b, '_, Pg>) -> serialize::Result {
        out.write_i32::<NetworkEndian>(*self)
            .map(|_| IsNull::No)
            .map_err(|e| Box::new(e) as Box<_>)
    }
}

impl ToSql<sql_types::BigInt, Pg> for i64 {
    fn to_sql<'a: 'b, 'b>(&'a self, out: &mut Output<'b, '_, Pg>) -> serialize::Result {
        out.write_i64::<NetworkEndian>(*self)
            .map(|_| IsNull::No)
            .map_err(|e| Box::new(e) as Box<_>)
    }
}

#[test]
fn i16_to_sql() {
    let mut buffer = Vec::new();
    let mut bytes = Output::test(&mut buffer);
    ToSql::<sql_types::SmallInt, Pg>::to_sql(&1i16, &mut bytes).unwrap();
    ToSql::<sql_types::SmallInt, Pg>::to_sql(&0i16, &mut bytes).unwrap();
    ToSql::<sql_types::SmallInt, Pg>::to_sql(&-1i16, &mut bytes).unwrap();
    assert_eq!(buffer, vec![0, 1, 0, 0, 255, 255]);
}

#[test]
fn i32_to_sql() {
    let mut buffer = Vec::new();
    let mut bytes = Output::test(&mut buffer);
    ToSql::<sql_types::Integer, Pg>::to_sql(&1i32, &mut bytes).unwrap();
    ToSql::<sql_types::Integer, Pg>::to_sql(&0i32, &mut bytes).unwrap();
    ToSql::<sql_types::Integer, Pg>::to_sql(&-1i32, &mut bytes).unwrap();
    assert_eq!(buffer, vec![0, 0, 0, 1, 0, 0, 0, 0, 255, 255, 255, 255]);
}

#[test]
fn i64_to_sql() {
    let mut buffer = Vec::new();
    let mut bytes = Output::test(&mut buffer);
    ToSql::<sql_types::BigInt, Pg>::to_sql(&1i64, &mut bytes).unwrap();
    ToSql::<sql_types::BigInt, Pg>::to_sql(&0i64, &mut bytes).unwrap();
    ToSql::<sql_types::BigInt, Pg>::to_sql(&-1i64, &mut bytes).unwrap();
    assert_eq!(
        buffer,
        vec![
            0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 255, 255, 255, 255, 255, 255, 255, 255,
        ]
    );
}
