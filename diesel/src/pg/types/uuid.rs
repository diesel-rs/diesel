use std::io::prelude::*;

use crate::deserialize::{self, FromSql, FromSqlRow};
use crate::expression::AsExpression;
use crate::pg::{Pg, PgValue};
use crate::serialize::{self, IsNull, Output, ToSql};
use crate::sql_types::Uuid;

#[derive(AsExpression, FromSqlRow)]
#[diesel(foreign_derive)]
#[diesel(sql_type = Uuid)]
#[allow(dead_code)]
struct UuidProxy(uuid::Uuid);

#[cfg(all(feature = "postgres_backend", feature = "uuid"))]
impl FromSql<Uuid, Pg> for uuid::Uuid {
    fn from_sql(value: PgValue<'_>) -> deserialize::Result<Self> {
        uuid::Uuid::from_slice(value.as_bytes()).map_err(Into::into)
    }
}

#[cfg(all(feature = "postgres_backend", feature = "uuid"))]
impl ToSql<Uuid, Pg> for uuid::Uuid {
    fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, Pg>) -> serialize::Result {
        out.write_all(self.as_bytes())
            .map(|_| IsNull::No)
            .map_err(Into::into)
    }
}

#[test]
fn uuid_to_sql() {
    use crate::query_builder::bind_collector::ByteWrapper;

    let mut buffer = Vec::new();
    let bytes = [
        0xFF_u8, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0x61, 0x62, 0x63, 0x64, 0x65, 0x66,
        0x31, 0x32,
    ];

    let test_uuid = uuid::Uuid::from_slice(&bytes).unwrap();
    let mut bytes = Output::test(ByteWrapper(&mut buffer));
    ToSql::<Uuid, Pg>::to_sql(&test_uuid, &mut bytes).unwrap();
    assert_eq!(&buffer, test_uuid.as_bytes());
}

#[test]
fn some_uuid_from_sql() {
    let bytes = [
        0xFF_u8, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0x61, 0x62, 0x63, 0x64, 0x65, 0x66,
        0x31, 0x32,
    ];
    let input_uuid = uuid::Uuid::from_slice(&bytes).unwrap();
    let output_uuid =
        FromSql::<Uuid, Pg>::from_sql(PgValue::for_test(input_uuid.as_bytes())).unwrap();
    assert_eq!(input_uuid, output_uuid);
}

#[test]
fn bad_uuid_from_sql() {
    let uuid = uuid::Uuid::from_sql(PgValue::for_test(b"boom"));
    assert!(uuid.is_err());
    // The error message changes slightly between different
    // uuid versions, so we just check on the relevant parts
    // The exact error messages are either:
    // "invalid bytes length: expected 16, found 4"
    // or
    // "invalid length: expected 16 bytes, found 4"
    let error_message = uuid.unwrap_err().to_string();
    assert!(error_message.starts_with("invalid"));
    assert!(error_message.contains("length"));
    assert!(error_message.contains("expected 16"));
    assert!(error_message.ends_with("found 4"));
}

#[test]
fn no_uuid_from_sql() {
    let uuid = uuid::Uuid::from_nullable_sql(None);
    assert_eq!(
        uuid.unwrap_err().to_string(),
        "Unexpected null for non-null column"
    );
}
