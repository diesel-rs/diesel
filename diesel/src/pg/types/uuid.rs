extern crate uuid;

use std::io::prelude::*;

use deserialize::{self, FromSql};
use pg::{Pg, PgValue};
use serialize::{self, IsNull, Output, ToSql};
use sql_types::Uuid;

#[derive(FromSqlRow, AsExpression)]
#[diesel(foreign_derive)]
#[sql_type = "Uuid"]
#[allow(dead_code)]
struct UuidProxy(uuid::Uuid);

impl FromSql<Uuid, Pg> for uuid::Uuid {
    fn from_sql(value: Option<PgValue>) -> deserialize::Result<Self> {
        let value = not_none!(value);
        uuid::Uuid::from_bytes(value.as_bytes()).map_err(Into::into)
    }
}

impl ToSql<Uuid, Pg> for uuid::Uuid {
    fn to_sql<W: Write>(&self, out: &mut Output<W, Pg>) -> serialize::Result {
        out.write_all(self.as_bytes())
            .map(|_| IsNull::No)
            .map_err(Into::into)
    }
}

#[test]
fn uuid_to_sql() {
    let mut bytes = Output::test();
    let test_uuid = uuid::Uuid::from_fields(0xFFFF_FFFF, 0xFFFF, 0xFFFF, b"abcdef12").unwrap();
    ToSql::<Uuid, Pg>::to_sql(&test_uuid, &mut bytes).unwrap();
    assert_eq!(bytes, test_uuid.as_bytes());
}

#[test]
fn some_uuid_from_sql() {
    use pg::StaticSqlType;
    let input_uuid = uuid::Uuid::from_fields(0xFFFF_FFFF, 0xFFFF, 0xFFFF, b"abcdef12").unwrap();
    let output_uuid =
        FromSql::<Uuid, Pg>::from_sql(Some(PgValue::new(input_uuid.as_bytes(), Uuid::OID)))
            .unwrap();
    assert_eq!(input_uuid, output_uuid);
}

#[test]
fn bad_uuid_from_sql() {
    use pg::StaticSqlType;
    let uuid = uuid::Uuid::from_sql(Some(PgValue::new(b"boom", Uuid::OID)));
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
