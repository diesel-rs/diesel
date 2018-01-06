extern crate uuid;

use std::io::prelude::*;
use std::error::Error;

use pg::Pg;
use types::{self, FromSql, IsNull, ToSql, ToSqlOutput, Uuid};

primitive_impls!(Uuid -> (uuid::Uuid, pg: (2950, 2951)));

#[derive(FromSqlRow)]
#[diesel(foreign_derive)]
#[allow(dead_code)]
struct UuidProxy(uuid::Uuid);

impl FromSql<types::Uuid, Pg> for uuid::Uuid {
    fn from_sql(bytes: Option<&[u8]>) -> Result<Self, Box<Error + Send + Sync>> {
        let bytes = not_none!(bytes);
        uuid::Uuid::from_bytes(bytes).map_err(|e| e.into())
    }
}

impl ToSql<types::Uuid, Pg> for uuid::Uuid {
    fn to_sql<W: Write>(
        &self,
        out: &mut ToSqlOutput<W, Pg>,
    ) -> Result<IsNull, Box<Error + Send + Sync>> {
        out.write_all(self.as_bytes())
            .map(|_| IsNull::No)
            .map_err(|e| Box::new(e) as Box<Error + Send + Sync>)
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
    let uuid: Result<uuid::Uuid, Box<Error + Send + Sync>> =
        FromSql::<types::Uuid, Pg>::from_sql(Some(b"boom"));
    assert_eq!(uuid.unwrap_err().description(), "UUID parse error");
}

#[test]
fn no_uuid_from_sql() {
    let uuid: Result<uuid::Uuid, Box<Error + Send + Sync>> =
        FromSql::<types::Uuid, Pg>::from_sql(None);
    assert_eq!(
        uuid.unwrap_err().description(),
        "Unexpected null for non-null column"
    );
}
