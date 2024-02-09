//! Support for JSON and `jsonb` values under PostgreSQL.

extern crate serde_json;

use std::io::prelude::*;

use crate::deserialize::{self, FromSql};
use crate::pg::{Pg, PgValue};
use crate::serialize::{self, IsNull, Output, ToSql};
use crate::sql_types;

#[cfg(all(feature = "postgres_backend", feature = "serde_json"))]
impl FromSql<sql_types::Json, Pg> for serde_json::Value {
    fn from_sql(value: PgValue<'_>) -> deserialize::Result<Self> {
        serde_json::from_slice(value.as_bytes()).map_err(|_| "Invalid Json".into())
    }
}

#[cfg(all(feature = "postgres_backend", feature = "serde_json"))]
impl ToSql<sql_types::Json, Pg> for serde_json::Value {
    fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, Pg>) -> serialize::Result {
        serde_json::to_writer(out, self)
            .map(|_| IsNull::No)
            .map_err(Into::into)
    }
}

#[cfg(all(feature = "postgres_backend", feature = "serde_json"))]
impl FromSql<sql_types::Jsonb, Pg> for serde_json::Value {
    fn from_sql(value: PgValue<'_>) -> deserialize::Result<Self> {
        let bytes = value.as_bytes();
        if bytes[0] != 1 {
            return Err("Unsupported JSONB encoding version".into());
        }
        serde_json::from_slice(&bytes[1..]).map_err(|_| "Invalid Json".into())
    }
}

#[cfg(all(feature = "postgres_backend", feature = "serde_json"))]
impl ToSql<sql_types::Jsonb, Pg> for serde_json::Value {
    fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, Pg>) -> serialize::Result {
        out.write_all(&[1])?;
        serde_json::to_writer(out, self)
            .map(|_| IsNull::No)
            .map_err(Into::into)
    }
}

#[cfg(test)]
mod tests {
    use crate::deserialize::FromSql;
    use crate::pg::{Pg, PgValue};
    use crate::query_builder::bind_collector::ByteWrapper;
    use crate::serialize::{Output, ToSql};
    use crate::sql_types;

    #[test]
    fn json_to_sql() {
        let mut buffer = Vec::new();
        let mut bytes = Output::test(ByteWrapper(&mut buffer));
        let test_json = serde_json::Value::Bool(true);
        ToSql::<sql_types::Json, Pg>::to_sql(&test_json, &mut bytes).unwrap();
        assert_eq!(buffer, b"true");
    }

    #[test]
    fn some_json_from_sql() {
        let input_json = b"true";
        let output_json: serde_json::Value =
            FromSql::<sql_types::Json, Pg>::from_sql(PgValue::for_test(input_json)).unwrap();
        assert_eq!(output_json, serde_json::Value::Bool(true));
    }

    #[test]
    fn bad_json_from_sql() {
        let uuid: Result<serde_json::Value, _> =
            FromSql::<sql_types::Json, Pg>::from_sql(PgValue::for_test(b"boom"));
        assert_eq!(uuid.unwrap_err().to_string(), "Invalid Json");
    }

    #[test]
    fn no_json_from_sql() {
        let uuid: Result<serde_json::Value, _> =
            FromSql::<sql_types::Json, Pg>::from_nullable_sql(None);
        assert_eq!(
            uuid.unwrap_err().to_string(),
            "Unexpected null for non-null column"
        );
    }

    #[test]
    fn jsonb_to_sql() {
        let mut buffer = Vec::new();
        let mut bytes = Output::test(ByteWrapper(&mut buffer));
        let test_json = serde_json::Value::Bool(true);
        ToSql::<sql_types::Jsonb, Pg>::to_sql(&test_json, &mut bytes).unwrap();
        assert_eq!(buffer, b"\x01true");
    }

    #[test]
    fn some_jsonb_from_sql() {
        let input_json = b"\x01true";
        let output_json: serde_json::Value =
            FromSql::<sql_types::Jsonb, Pg>::from_sql(PgValue::for_test(input_json)).unwrap();
        assert_eq!(output_json, serde_json::Value::Bool(true));
    }

    #[test]
    fn bad_jsonb_from_sql() {
        let uuid: Result<serde_json::Value, _> =
            FromSql::<sql_types::Jsonb, Pg>::from_sql(PgValue::for_test(b"\x01boom"));
        assert_eq!(uuid.unwrap_err().to_string(), "Invalid Json");
    }

    #[test]
    fn bad_jsonb_version_from_sql() {
        let uuid: Result<serde_json::Value, _> =
            FromSql::<sql_types::Jsonb, Pg>::from_sql(PgValue::for_test(b"\x02true"));
        assert_eq!(
            uuid.unwrap_err().to_string(),
            "Unsupported JSONB encoding version"
        );
    }

    #[test]
    fn no_jsonb_from_sql() {
        let uuid: Result<serde_json::Value, _> =
            FromSql::<sql_types::Jsonb, Pg>::from_nullable_sql(None);
        assert_eq!(
            uuid.unwrap_err().to_string(),
            "Unexpected null for non-null column"
        );
    }
}
