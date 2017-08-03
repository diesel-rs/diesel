//! Support for JSON and `jsonb` values under PostgreSQL.

extern crate serde_json;

use std::io::prelude::*;
use std::error::Error;

use pg::Pg;
use types::{self, ToSql, ToSqlOutput, IsNull, FromSql, Json, Jsonb};

// The OIDs used to identify `json` and `jsonb` are not documented anywhere
// obvious, but they are discussed on various PostgreSQL mailing lists,
// including:
//
// https://www.postgresql.org/message-id/CA+mi_8Yv2SVOdhAtx-4CbpzoDtaJGkf8QvnushdF8bMgySAbYg@mail.gmail.com
// https://www.postgresql.org/message-id/CA+mi_8bd_g-MDPMwa88w0HXfjysaLFcrCza90+KL9zpRGbxKWg@mail.gmail.com
primitive_impls!(Json -> (serde_json::Value, pg: (114, 199)));
primitive_impls!(Jsonb -> (serde_json::Value, pg: (3802, 3807)));

impl FromSql<types::Json, Pg> for serde_json::Value {
    fn from_sql(bytes: Option<&[u8]>) -> Result<Self, Box<Error+Send+Sync>> {
        let bytes = not_none!(bytes);
        serde_json::from_slice(bytes)
            .map_err(|e| Box::new(e) as Box<Error+Send+Sync>)
    }
}

impl ToSql<types::Json, Pg> for serde_json::Value {
    fn to_sql<W: Write>(&self, out: &mut ToSqlOutput<W, Pg>) -> Result<IsNull, Box<Error+Send+Sync>> {
        serde_json::to_writer(out, self)
            .map(|_| IsNull::No)
            .map_err(|e| Box::new(e) as Box<Error+Send+Sync>)
    }
}

impl FromSql<types::Jsonb, Pg> for serde_json::Value {
    fn from_sql(bytes: Option<&[u8]>) -> Result<Self, Box<Error+Send+Sync>> {
        let bytes = not_none!(bytes);
        if bytes[0] != 1 {
            return Err("Unsupported JSONB encoding version".into());
        }
        serde_json::from_slice(&bytes[1..])
            .map_err(|e| Box::new(e) as Box<Error+Send+Sync>)
    }
}

impl ToSql<types::Jsonb, Pg> for serde_json::Value {
    fn to_sql<W: Write>(&self, out: &mut ToSqlOutput<W, Pg>) -> Result<IsNull, Box<Error+Send+Sync>> {
        try!(out.write_all(&[1]));
        serde_json::to_writer(out, self)
            .map(|_| IsNull::No)
            .map_err(|e| Box::new(e) as Box<Error+Send+Sync>)
    }
}

#[test]
fn json_to_sql() {
    let mut bytes = ToSqlOutput::test();
    let test_json = serde_json::Value::Bool(true);
    ToSql::<types::Json, Pg>::to_sql(&test_json, &mut bytes).unwrap();
    assert_eq!(bytes, b"true");
}

#[test]
fn some_json_from_sql() {
    let input_json = b"true";
    let output_json: serde_json::Value =
        FromSql::<types::Json, Pg>::from_sql(Some(input_json)).unwrap();
    assert_eq!(output_json, serde_json::Value::Bool(true));
}

#[test]
fn bad_json_from_sql() {
    let uuid: Result<serde_json::Value, Box<Error+Send+Sync>> =
        FromSql::<types::Json, Pg>::from_sql(Some(b"boom"));
    assert_eq!(uuid.unwrap_err().description(), "JSON error");
}

#[test]
fn no_json_from_sql() {
    let uuid: Result<serde_json::Value, Box<Error+Send+Sync>> =
        FromSql::<types::Json, Pg>::from_sql(None);
    assert_eq!(uuid.unwrap_err().description(), "Unexpected null for non-null column");
}

#[test]
fn jsonb_to_sql() {
    let mut bytes = ToSqlOutput::test();
    let test_json = serde_json::Value::Bool(true);
    ToSql::<types::Jsonb, Pg>::to_sql(&test_json, &mut bytes).unwrap();
    assert_eq!(bytes, b"\x01true");
}

#[test]
fn some_jsonb_from_sql() {
    let input_json = b"\x01true";
    let output_json: serde_json::Value =
        FromSql::<types::Jsonb, Pg>::from_sql(Some(input_json)).unwrap();
    assert_eq!(output_json, serde_json::Value::Bool(true));
}

#[test]
fn bad_jsonb_from_sql() {
    let uuid: Result<serde_json::Value, Box<Error+Send+Sync>> =
        FromSql::<types::Jsonb, Pg>::from_sql(Some(b"\x01boom"));
    assert_eq!(uuid.unwrap_err().description(), "JSON error");
}

#[test]
fn bad_jsonb_version_from_sql() {
    let uuid: Result<serde_json::Value, Box<Error+Send+Sync>> =
        FromSql::<types::Jsonb, Pg>::from_sql(Some(b"\x02true"));
    assert_eq!(uuid.unwrap_err().description(), "Unsupported JSONB encoding version");
}

#[test]
fn no_jsonb_from_sql() {
    let uuid: Result<serde_json::Value, Box<Error+Send+Sync>> =
        FromSql::<types::Jsonb, Pg>::from_sql(None);
    assert_eq!(uuid.unwrap_err().description(), "Unexpected null for non-null column");
}
