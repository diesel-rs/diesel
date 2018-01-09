extern crate uuid;

use std::io::prelude::*;

use pg::Pg;
use types::{self, FromSql, IsNull, ToSql, ToSqlOutput, Uuid};
use {deserialize, serialize};

#[derive(FromSqlRow, AsExpression)]
#[diesel(foreign_derive)]
#[sql_type = "Uuid"]
#[allow(dead_code)]
struct UuidProxy(uuid::Uuid);

impl FromSql<types::Uuid, Pg> for uuid::Uuid {
    fn from_sql(bytes: Option<&[u8]>) -> deserialize::Result<Self> {
        let bytes = not_none!(bytes);
        uuid::Uuid::from_bytes(bytes).map_err(|e| e.into())
    }
}

impl ToSql<types::Uuid, Pg> for uuid::Uuid {
    fn to_sql<W: Write>(&self, out: &mut ToSqlOutput<W, Pg>) -> serialize::Result {
        out.write_all(self.as_bytes())
            .map(|_| IsNull::No)
            .map_err(Into::into)
    }
}

#[test]
fn uuid_to_sql() {
    let mut bytes = ToSqlOutput::test();
    let test_uuid = uuid::Uuid::from_fields(4_294_967_295, 65_535, 65_535, b"abcdef12").unwrap();
    ToSql::<types::Uuid, Pg>::to_sql(&test_uuid, &mut bytes).unwrap();
    assert_eq!(bytes, test_uuid.as_bytes());
}

#[test]
fn some_uuid_from_sql() {
    let input_uuid = uuid::Uuid::from_fields(4_294_967_295, 65_535, 65_535, b"abcdef12").unwrap();
    let output_uuid = FromSql::<types::Uuid, Pg>::from_sql(Some(input_uuid.as_bytes())).unwrap();
    assert_eq!(input_uuid, output_uuid);
}

#[test]
fn bad_uuid_from_sql() {
    let uuid = uuid::Uuid::from_sql(Some(b"boom"));
    assert_eq!(uuid.unwrap_err().description(), "UUID parse error");
}

#[test]
fn no_uuid_from_sql() {
    let uuid = uuid::Uuid::from_sql(None);
    assert_eq!(
        uuid.unwrap_err().description(),
        "Unexpected null for non-null column"
    );
}
