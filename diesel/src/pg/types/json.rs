//! Support for JSON and `jsonb` values under PostgreSQL.

extern crate serde_json;

use std::io::prelude::*;

use deserialize::{self, FromSql};
use pg::{Pg, PgValue};
use serialize::{self, IsNull, Output, ToSql};
use sql_types;

#[allow(dead_code)]
mod foreign_derives {
    use super::serde_json;
    use sql_types::{Json, Jsonb};

    #[derive(FromSqlRow, AsExpression)]
    #[diesel(foreign_derive)]
    #[sql_type = "Json"]
    #[sql_type = "Jsonb"]
    struct SerdeJsonValueProxy(serde_json::Value);
}

impl FromSql<sql_types::Json, Pg> for serde_json::Value {
    fn from_sql(value: Option<PgValue>) -> deserialize::Result<Self> {
        let value = not_none!(value);
        serde_json::from_slice(value.as_bytes()).map_err(Into::into)
    }
}

impl ToSql<sql_types::Json, Pg> for serde_json::Value {
    fn to_sql<W: Write>(&self, out: &mut Output<W, Pg>) -> serialize::Result {
        serde_json::to_writer(out, self)
            .map(|_| IsNull::No)
            .map_err(Into::into)
    }
}

impl FromSql<sql_types::Jsonb, Pg> for serde_json::Value {
    fn from_sql(value: Option<PgValue>) -> deserialize::Result<Self> {
        let value = not_none!(value);
        let bytes = value.as_bytes();
        if bytes[0] != 1 {
            return Err("Unsupported JSONB encoding version".into());
        }
        serde_json::from_slice(&bytes[1..]).map_err(Into::into)
    }
}

impl ToSql<sql_types::Jsonb, Pg> for serde_json::Value {
    fn to_sql<W: Write>(&self, out: &mut Output<W, Pg>) -> serialize::Result {
        out.write_all(&[1])?;
        serde_json::to_writer(out, self)
            .map(|_| IsNull::No)
            .map_err(Into::into)
    }
}

#[test]
fn json_to_sql() {
    let mut bytes = Output::test();
    let test_json = serde_json::Value::Bool(true);
    ToSql::<sql_types::Json, Pg>::to_sql(&test_json, &mut bytes).unwrap();
    assert_eq!(bytes, b"true");
}

#[test]
fn some_json_from_sql() {
    use pg::StaticSqlType;
    let input_json = b"true";
    let output_json: serde_json::Value = FromSql::<sql_types::Json, Pg>::from_sql(Some(
        PgValue::new(input_json, sql_types::Json::OID),
    ))
    .unwrap();
    assert_eq!(output_json, serde_json::Value::Bool(true));
}

#[test]
fn bad_json_from_sql() {
    use pg::StaticSqlType;
    let uuid: Result<serde_json::Value, _> =
        FromSql::<sql_types::Json, Pg>::from_sql(Some(PgValue::new(b"boom", sql_types::Json::OID)));
    assert_eq!(uuid.unwrap_err().description(), "JSON error");
}

#[test]
fn no_json_from_sql() {
    let uuid: Result<serde_json::Value, _> = FromSql::<sql_types::Json, Pg>::from_sql(None);
    assert_eq!(
        uuid.unwrap_err().description(),
        "Unexpected null for non-null column"
    );
}

#[test]
fn jsonb_to_sql() {
    let mut bytes = Output::test();
    let test_json = serde_json::Value::Bool(true);
    ToSql::<sql_types::Jsonb, Pg>::to_sql(&test_json, &mut bytes).unwrap();
    assert_eq!(bytes, b"\x01true");
}

#[test]
fn some_jsonb_from_sql() {
    use pg::StaticSqlType;
    let input_json = b"\x01true";
    let output_json: serde_json::Value = FromSql::<sql_types::Jsonb, Pg>::from_sql(Some(
        PgValue::new(input_json, sql_types::Jsonb::OID),
    ))
    .unwrap();
    assert_eq!(output_json, serde_json::Value::Bool(true));
}

#[test]
fn bad_jsonb_from_sql() {
    use pg::StaticSqlType;
    let uuid: Result<serde_json::Value, _> = FromSql::<sql_types::Jsonb, Pg>::from_sql(Some(
        PgValue::new(b"\x01boom", sql_types::Jsonb::OID),
    ));
    assert_eq!(uuid.unwrap_err().description(), "JSON error");
}

#[test]
fn bad_jsonb_version_from_sql() {
    use pg::StaticSqlType;
    let uuid: Result<serde_json::Value, _> = FromSql::<sql_types::Jsonb, Pg>::from_sql(Some(
        PgValue::new(b"\x02true", sql_types::Jsonb::OID),
    ));
    assert_eq!(
        uuid.unwrap_err().description(),
        "Unsupported JSONB encoding version"
    );
}

#[test]
fn no_jsonb_from_sql() {
    let uuid: Result<serde_json::Value, _> = FromSql::<sql_types::Jsonb, Pg>::from_sql(None);
    assert_eq!(
        uuid.unwrap_err().description(),
        "Unexpected null for non-null column"
    );
}
