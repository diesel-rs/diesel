extern crate serde_json;

use crate::deserialize::{self, FromSql};
use crate::serialize::{self, IsNull, Output, ToSql};
use crate::sql_types;
use crate::sqlite::{Sqlite, SqliteValue};

#[cfg(all(feature = "sqlite", feature = "serde_json"))]
impl FromSql<sql_types::Json, Sqlite> for serde_json::Value {
    fn from_sql(value: SqliteValue<'_, '_, '_>) -> deserialize::Result<Self> {
        serde_json::from_str(value.read_text()).map_err(|_| "Invalid Json".into())
    }
}

#[cfg(all(feature = "sqlite", feature = "serde_json"))]
impl ToSql<sql_types::Json, Sqlite> for serde_json::Value {
    fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, Sqlite>) -> serialize::Result {
        out.set_value(serde_json::to_string(self)?);
        Ok(IsNull::No)
    }
}

#[cfg(all(feature = "sqlite", feature = "serde_json"))]
impl FromSql<sql_types::Jsonb, Sqlite> for serde_json::Value {
    fn from_sql(value: SqliteValue<'_, '_, '_>) -> deserialize::Result<Self> {
        let bytes = value.read_blob();
        if bytes[0] != 1 {
            return Err("Unsupported JSONB encoding version".into());
        }
        serde_json::from_slice(&bytes[1..]).map_err(|_| "Invalid Json".into())
    }
}

#[cfg(all(feature = "sqlite", feature = "serde_json"))]
impl ToSql<sql_types::Jsonb, Sqlite> for serde_json::Value {
    fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, Sqlite>) -> serialize::Result {
        out.set_value(serde_json::to_string(self)?.into_bytes());
        Ok(IsNull::No)
    }
}

#[cfg(test)]
mod tests {
    use crate::deserialize::FromSql;
    use crate::dsl::sql;
    use crate::prelude::*;
    use crate::select;
    use crate::serialize::{Output, ToSql};
    use crate::sql_types;
    use crate::sql_types::{Json, Text};
    use crate::sqlite::connection::SqliteBindCollector;
    use crate::sqlite::Sqlite;
    use crate::sqlite::SqliteBindValue;
    use crate::sqlite::SqliteValue;
    use crate::test_helpers::connection;
    use serde_json::json;

    // #[test]
    // fn json_to_sql() {
    //     let buffer = SqliteBindValue::from(0i32);
    //     let mut out = Output::<'_, 'static, Sqlite>::test(buffer);
    //     let test_json = serde_json::Value::Bool(true);
    //     ToSql::<sql_types::Json, Sqlite>::to_sql(&test_json, &mut out).unwrap();
    //     assert_eq!(buffer.inner, SqliteBindValue::from(1i32).inner);
    // }

    // #[test]
    // fn json_to_sql() {
    //     crate::table! {
    //         #[allow(unused_parens)]
    //         test_insert_json_into_table_as_text(id) {
    //             id -> Integer,
    //             json -> Text,
    //         }
    //     }
    //     let conn = &mut connection();
    //     crate::sql_query(
    //         "CREATE TABLE test_insert_json_into_table_as_text(id INTEGER PRIMARY KEY, json TEXT);",
    //     )
    //     .execute(conn)
    //     .unwrap();

    //     let value = json!(true);

    //     crate::insert_into(test_insert_json_into_table_as_text::table)
    //         .values((
    //             test_insert_json_into_table_as_text::id.eq(1),
    //             test_insert_json_into_table_as_text::json.eq(value),
    //         ))
    //         .execute(conn)
    //         .unwrap();
    // }

    #[test]
    fn some_json_from_sql() {
        let input_json = b"true";
        let output_json: serde_json::Value =
            FromSql::<sql_types::Json, Sqlite>::from_sql(SqliteValue::for_test(input_json))
                .unwrap();
        assert_eq!(output_json, serde_json::Value::Bool(true));
    }

    // #[test]
    // fn bad_json_from_sql() {
    //     let uuid: Result<serde_json::Value, _> =
    //         FromSql::<sql_types::Json, Sqlite>::from_sql(SqliteValue::for_test(b"boom"));
    //     assert_eq!(uuid.unwrap_err().to_string(), "Invalid Json");
    // }

    // #[test]
    // fn no_json_from_sql() {
    //     let uuid: Result<serde_json::Value, _> =
    //         FromSql::<sql_types::Json, Sqlite>::from_nullable_sql(None);
    //     assert_eq!(
    //         uuid.unwrap_err().to_string(),
    //         "Unexpected null for non-null column"
    //     );
    // }

    // #[test]
    // fn jsonb_to_sql() {
    //     let mut buffer = Vec::new();
    //     let mut bytes = Output::test(ByteWrapper(&mut buffer));
    //     let test_json = serde_json::Value::Bool(true);
    //     ToSql::<sql_types::Jsonb, Sqlite>::to_sql(&test_json, &mut bytes).unwrap();
    //     assert_eq!(buffer, b"\x01true");
    // }

    // #[test]
    // fn some_jsonb_from_sql() {
    //     let input_json = b"\x01true";
    //     let output_json: serde_json::Value =
    //         FromSql::<sql_types::Jsonb, Sqlite>::from_sql(SqliteValue::for_test(input_json))
    //             .unwrap();
    //     assert_eq!(output_json, serde_json::Value::Bool(true));
    // }

    // #[test]
    // fn bad_jsonb_from_sql() {
    //     let uuid: Result<serde_json::Value, _> =
    //         FromSql::<sql_types::Jsonb, Sqlite>::from_sql(SqliteValue::for_test(b"\x01boom"));
    //     assert_eq!(uuid.unwrap_err().to_string(), "Invalid Json");
    // }

    // #[test]
    // fn bad_jsonb_version_from_sql() {
    //     let uuid: Result<serde_json::Value, _> =
    //         FromSql::<sql_types::Jsonb, Sqlite>::from_sql(SqliteValue::for_test(b"\x02true"));
    //     assert_eq!(
    //         uuid.unwrap_err().to_string(),
    //         "Unsupported JSONB encoding version"
    //     );
    // }

    // #[test]
    // fn no_jsonb_from_sql() {
    //     let uuid: Result<serde_json::Value, _> =
    //         FromSql::<sql_types::Jsonb, Sqlite>::from_nullable_sql(None);
    //     assert_eq!(
    //         uuid.unwrap_err().to_string(),
    //         "Unexpected null for non-null column"
    //     );
    // }
}
