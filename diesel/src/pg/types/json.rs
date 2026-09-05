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
        let first_byte = bytes
            .first()
            .ok_or("Received an empty response from the server")?;

        if *first_byte != 1 {
            return Err("Unsupported JSONB encoding version".into());
        }
        // That's an empty slice if there is only
        // one response byte
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

    #[diesel_test_helper::test]
    fn regression_json_float_survives_a_round_trip() {
        // `serde_json` prints the shortest text that round trips, and parses it
        // back approximately unless `float_roundtrip` is on, so about three in
        // ten floats used to come back as the neighbouring double
        for float in [
            8.829872855928286e-308f64,
            -0.20221894534048165,
            1.7383394626966921e-307,
            6.178787134922198e305,
            0.1,
            f64::MIN_POSITIVE,
            f64::MAX,
        ] {
            let value = serde_json::Value::from(float);

            let mut buffer = Vec::new();
            let mut bytes = Output::test(ByteWrapper(&mut buffer));
            ToSql::<sql_types::Json, Pg>::to_sql(&value, &mut bytes).unwrap();
            let json: serde_json::Value =
                FromSql::<sql_types::Json, Pg>::from_sql(PgValue::for_test(&buffer)).unwrap();
            assert_eq!(json.as_f64().map(f64::to_bits), Some(float.to_bits()));

            let mut buffer = Vec::new();
            let mut bytes = Output::test(ByteWrapper(&mut buffer));
            ToSql::<sql_types::Jsonb, Pg>::to_sql(&value, &mut bytes).unwrap();
            let jsonb: serde_json::Value =
                FromSql::<sql_types::Jsonb, Pg>::from_sql(PgValue::for_test(&buffer)).unwrap();
            assert_eq!(jsonb.as_f64().map(f64::to_bits), Some(float.to_bits()));
        }
    }

    #[diesel_test_helper::test]
    fn json_to_sql() {
        let mut buffer = Vec::new();
        let mut bytes = Output::test(ByteWrapper(&mut buffer));
        let test_json = serde_json::Value::Bool(true);
        ToSql::<sql_types::Json, Pg>::to_sql(&test_json, &mut bytes).unwrap();
        assert_eq!(buffer, b"true");
    }

    #[diesel_test_helper::test]
    fn some_json_from_sql() {
        let input_json = b"true";
        let output_json: serde_json::Value =
            FromSql::<sql_types::Json, Pg>::from_sql(PgValue::for_test(input_json)).unwrap();
        assert_eq!(output_json, serde_json::Value::Bool(true));
    }

    #[diesel_test_helper::test]
    fn bad_json_from_sql() {
        let uuid: Result<serde_json::Value, _> =
            FromSql::<sql_types::Json, Pg>::from_sql(PgValue::for_test(b"boom"));
        assert_eq!(uuid.unwrap_err().to_string(), "Invalid Json");
    }

    #[diesel_test_helper::test]
    fn no_json_from_sql() {
        let uuid: Result<serde_json::Value, _> =
            FromSql::<sql_types::Json, Pg>::from_nullable_sql(None);
        assert_eq!(
            uuid.unwrap_err().to_string(),
            "Unexpected null for non-null column"
        );
    }

    #[diesel_test_helper::test]
    fn jsonb_to_sql() {
        let mut buffer = Vec::new();
        let mut bytes = Output::test(ByteWrapper(&mut buffer));
        let test_json = serde_json::Value::Bool(true);
        ToSql::<sql_types::Jsonb, Pg>::to_sql(&test_json, &mut bytes).unwrap();
        assert_eq!(buffer, b"\x01true");
    }

    #[diesel_test_helper::test]
    fn some_jsonb_from_sql() {
        let input_json = b"\x01true";
        let output_json: serde_json::Value =
            FromSql::<sql_types::Jsonb, Pg>::from_sql(PgValue::for_test(input_json)).unwrap();
        assert_eq!(output_json, serde_json::Value::Bool(true));
    }

    #[diesel_test_helper::test]
    fn bad_jsonb_from_sql() {
        let uuid: Result<serde_json::Value, _> =
            FromSql::<sql_types::Jsonb, Pg>::from_sql(PgValue::for_test(b"\x01boom"));
        assert_eq!(uuid.unwrap_err().to_string(), "Invalid Json");
    }

    #[diesel_test_helper::test]
    fn bad_jsonb_version_from_sql() {
        let uuid: Result<serde_json::Value, _> =
            FromSql::<sql_types::Jsonb, Pg>::from_sql(PgValue::for_test(b"\x02true"));
        assert_eq!(
            uuid.unwrap_err().to_string(),
            "Unsupported JSONB encoding version"
        );
    }

    #[diesel_test_helper::test]
    fn no_jsonb_from_sql() {
        let uuid: Result<serde_json::Value, _> =
            FromSql::<sql_types::Jsonb, Pg>::from_nullable_sql(None);
        assert_eq!(
            uuid.unwrap_err().to_string(),
            "Unexpected null for non-null column"
        );
    }
}
