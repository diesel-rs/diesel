extern crate uuidv07 as uuid;

use std::io::prelude::*;

use deserialize::{self, FromSql};
use mysql::Mysql;
use serialize::{self, IsNull, Output, ToSql};
use sql_types::Binary;

#[derive(FromSqlRow, AsExpression)]
#[diesel(foreign_derive)]
#[sql_type = "Binary"]
#[allow(dead_code)]
struct UuidProxy(uuid::Uuid);

impl FromSql<Binary, Mysql> for uuid::Uuid {
    fn from_sql(bytes: Option<&[u8]>) -> deserialize::Result<Self> {
        let bytes = not_none!(bytes);
        uuid::Uuid::from_slice(bytes).map_err(Into::into)
    }
}

impl ToSql<Binary, Mysql> for uuid::Uuid {
    fn to_sql<W: Write>(&self, out: &mut Output<W, Mysql>) -> serialize::Result {
        out.write_all(self.as_bytes())
            .map(|_| IsNull::No)
            .map_err(Into::into)
    }
}

#[test]
fn uuid_to_sql() {
    let mut bytes = Output::test();
    let test_uuid = uuid::Uuid::from_fields(0xFFFF_FFFF, 0xFFFF, 0xFFFF, b"abcdef12").unwrap();
    ToSql::<Uuid, Mysql>::to_sql(&test_uuid, &mut bytes).unwrap();
    assert_eq!(bytes, test_uuid.as_bytes());
}

#[test]
fn some_uuid_from_sql() {
    let input_uuid = uuid::Uuid::from_fields(0xFFFF_FFFF, 0xFFFF, 0xFFFF, b"abcdef12").unwrap();
    let output_uuid = FromSql::<Uuid, Mysql>::from_sql(Some(input_uuid.as_bytes())).unwrap();
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
