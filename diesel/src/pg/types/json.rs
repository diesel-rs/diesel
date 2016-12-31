//! Support for JSON and `jsonb` values under PostgreSQL.

extern crate serde_json;

use std::io::prelude::*;
use std::error::Error;

use pg::Pg;
use types::{self, ToSql, IsNull, FromSql};

primitive_impls!(Json -> (serde_json::Value, pg: (114, 199)));

impl FromSql<types::Json, Pg> for serde_json::Value {
    fn from_sql(bytes: Option<&[u8]>) -> Result<Self, Box<Error+Send+Sync>> {
        let bytes = not_none!(bytes);
        serde_json::from_slice(&bytes)
            .map_err(|e| Box::new(e) as Box<Error+Send+Sync>)
    }
}

impl ToSql<types::Json, Pg> for serde_json::Value {
    fn to_sql<W: Write>(&self, out: &mut W) -> Result<IsNull, Box<Error+Send+Sync>> {
        serde_json::to_writer(out, self)
            .map(|_| IsNull::No)
            .map_err(|e| Box::new(e) as Box<Error+Send+Sync>)
    }
}

#[test]
fn json_to_sql() {
    let mut bytes = vec![];
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
    assert_eq!(uuid.unwrap_err().description(), "syntax error");
}

#[test]
fn no_json_from_sql() {
    let uuid: Result<serde_json::Value, Box<Error+Send+Sync>> =
        FromSql::<types::Json, Pg>::from_sql(None);
    assert_eq!(uuid.unwrap_err().description(), "Unexpected null for non-null column");
}
