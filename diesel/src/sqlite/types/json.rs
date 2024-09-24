extern crate serde_json;

use crate::deserialize::{self, FromSql};
use crate::serialize::{self, IsNull, Output, ToSql};
use crate::sql_types;
use crate::sqlite::{Sqlite, SqliteValue};

impl FromSql<sql_types::Json, Sqlite> for serde_json::Value {
    fn from_sql(value: SqliteValue<'_, '_, '_>) -> deserialize::Result<Self> {
        serde_json::from_str(value.read_text()).map_err(|_| "Invalid Json".into())
    }
}

impl ToSql<sql_types::Json, Sqlite> for serde_json::Value {
    fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, Sqlite>) -> serialize::Result {
        out.set_value(serde_json::to_string(self)?);
        Ok(IsNull::No)
    }
}

impl FromSql<sql_types::Jsonb, Sqlite> for serde_json::Value {
    fn from_sql(value: SqliteValue<'_, '_, '_>) -> deserialize::Result<Self> {
        let bytes = value.read_blob();

        // Ensure we have at least one byte for the version check
        if bytes.is_empty() {
            return Err("Empty blob cannot be decoded as JSONB".into());
        }

        // Parse the first byte to determine the header size and the type
        let (element_type, payload_size, remaining_bytes) = read_jsonb_header(bytes)?;

        // Parse the payload based on the element type
        read_jsonb_element(element_type, payload_size, remaining_bytes)
    }
}

impl ToSql<sql_types::Jsonb, Sqlite> for serde_json::Value {
    fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, Sqlite>) -> serialize::Result {
        out.set_value(serde_json::to_string(self)?.into_bytes());
        Ok(IsNull::No)
    }
}

// Parse the header, including both the element type and the payload size
fn read_jsonb_header(bytes: &[u8]) -> deserialize::Result<(u8, usize, &[u8])> {
    let first_byte = bytes[0];

    // The upper 4 bits of the first byte determine the header size or the payload size directly
    let header_size_encoding = (first_byte & 0xf0) >> 4;
    let element_type = first_byte & 0x0f;

    let (payload_size, remaining_bytes) = match header_size_encoding {
        0x00..=0x0b => {
            // If upper bits are between 0 and 11, payload size is stored in those bits directly
            let payload_size = header_size_encoding as usize;
            (payload_size, &bytes[1..])
        }
        0x0c => {
            // Upper bits are 12, so payload size is in the next byte (2-byte header)
            if bytes.len() < 2 {
                return Err("Invalid JSONB: insufficient bytes for payload size".into());
            }
            let payload_size = bytes[1] as usize;
            (payload_size, &bytes[2..])
        }
        0x0d => {
            // Upper bits are 13, so payload size is in the next 2 bytes (3-byte header)
            if bytes.len() < 3 {
                return Err("Invalid JSONB: insufficient bytes for payload size".into());
            }
            let payload_size = u16::from_be_bytes([bytes[1], bytes[2]]) as usize;
            (payload_size, &bytes[3..])
        }
        0x0e => {
            // Upper bits are 14, so payload size is in the next 4 bytes (5-byte header)
            if bytes.len() < 5 {
                return Err("Invalid JSONB: insufficient bytes for payload size".into());
            }
            let payload_size =
                u32::from_be_bytes([bytes[1], bytes[2], bytes[3], bytes[4]]) as usize;
            (payload_size, &bytes[5..])
        }
        0x0f => {
            // Upper bits are 15, so payload size is in the next 8 bytes (9-byte header)
            if bytes.len() < 9 {
                return Err("Invalid JSONB: insufficient bytes for payload size".into());
            }
            let payload_size = u64::from_be_bytes([
                bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7], bytes[8],
            ]) as usize;
            (payload_size, &bytes[9..])
        }
        _ => return Err("Invalid header encoding".into()),
    };

    Ok((element_type, payload_size, remaining_bytes))
}

// Parse the actual element based on its type and payload size
fn read_jsonb_element(
    element_type: u8,
    payload_size: usize,
    bytes: &[u8],
) -> deserialize::Result<serde_json::Value> {
    match element_type {
        0x00 => Ok(serde_json::Value::Null),            // NULL
        0x01 => Ok(serde_json::Value::Bool(true)),      // TRUE
        0x02 => Ok(serde_json::Value::Bool(false)),     // FALSE
        0x03 => read_jsonb_integer(bytes),              // INT
        0x04 => read_jsonb_float(bytes),                // FLOAT
        0x05 => read_jsonb_text(payload_size, bytes),   // TEXT
        0x06 => read_jsonb_array(bytes),                // ARRAY
        0x07 => read_jsonb_object(payload_size, bytes), // OBJECT
        _ => Err(format!("Unsupported or reserved JSONB type: {}", element_type).into()),
    }
}

// Parse a JSONB integer
fn read_jsonb_integer(bytes: &[u8]) -> deserialize::Result<serde_json::Value> {
    let int_str = std::str::from_utf8(bytes).map_err(|_| "Invalid UTF-8 in JSONB integer")?;
    let int_value = int_str
        .parse::<i64>()
        .map_err(|_| "Failed to parse JSONB integer")?;
    Ok(serde_json::Value::Number(serde_json::Number::from(
        int_value,
    )))
}

// Parse a JSONB float
fn read_jsonb_float(bytes: &[u8]) -> deserialize::Result<serde_json::Value> {
    let float_str = std::str::from_utf8(bytes).map_err(|_| "Invalid UTF-8 in JSONB float")?;
    let float_value = float_str
        .parse::<f64>()
        .map_err(|_| "Failed to parse JSONB float")?;
    Ok(serde_json::Value::Number(
        serde_json::Number::from_f64(float_value).unwrap(),
    ))
}

// Parse a JSONB text (string)
fn read_jsonb_text(payload_size: usize, bytes: &[u8]) -> deserialize::Result<serde_json::Value> {
    if bytes.len() < payload_size {
        return Err("Invalid JSONB text: insufficient bytes".into());
    }
    let text =
        std::str::from_utf8(&bytes[..payload_size]).map_err(|_| "Invalid UTF-8 in JSONB text")?;
    Ok(serde_json::Value::String(text.to_string()))
}

// Parse a JSONB array (recursive parsing)
fn read_jsonb_array(bytes: &[u8]) -> deserialize::Result<serde_json::Value> {
    let mut elements = Vec::new();
    let mut remaining_bytes = bytes;

    while !remaining_bytes.is_empty() {
        let (element_type, element_size, rest) = read_jsonb_header(remaining_bytes)?;
        let element = read_jsonb_element(element_type, element_size, rest)?;
        elements.push(element);
        remaining_bytes = &remaining_bytes[element_size + 1..]; // Adjust for header
    }

    Ok(serde_json::Value::Array(elements))
}

// Parse a JSONB object (recursive parsing)
fn read_jsonb_object(payload_size: usize, bytes: &[u8]) -> deserialize::Result<serde_json::Value> {
    let mut object = serde_json::Map::new();
    let mut remaining_bytes = bytes;
    let mut total_read = 0;

    // Loop through the object key-value pairs
    while total_read < payload_size {
        // Read the key header
        let (key_type, key_size, rest) = read_jsonb_header(remaining_bytes)?;

        // Ensure the key is a valid string type (TEXT, TEXTJ, TEXT5, or TEXTRAW)
        match key_type {
            0x05 | 0x06 | 0x07 | 0x08 => {
                // Valid string types: TEXT, TEXTJ, TEXT5, TEXTRAW
                let key = read_jsonb_text(key_size, rest)?
                    .as_str()
                    .ok_or("Invalid object key in JSONB")?
                    .to_string();

                // Move the remaining bytes pointer past the key
                remaining_bytes = &rest[key_size..];

                // Read the value header
                let (value_type, value_size, rest_after_value) =
                    read_jsonb_header(remaining_bytes)?;

                // Parse the value based on its type
                let value = read_jsonb_element(value_type, value_size, rest_after_value)?;

                // Insert the key-value pair into the object map
                object.insert(key, value);

                // Move the remaining bytes pointer past the value
                remaining_bytes = &remaining_bytes[value_size + 1..];
                total_read += key_size + value_size + 2; // Adjust total read for key and value size and headers
            }
            _ => {
                return Err(format!("Invalid JSONB object key type: {}", key_type).into());
            }
        }
    }

    Ok(serde_json::Value::Object(object))
}

#[cfg(test)]
mod tests {
    use crate::deserialize::FromSql;
    use crate::dsl::sql;
    use crate::prelude::*;
    use crate::select;
    use crate::serialize::{Output, ToSql};
    use crate::sql_types;
    use crate::sql_types::Jsonb;
    use crate::sql_types::{Json, Text};
    use crate::sqlite::connection::SqliteBindCollector;
    use crate::sqlite::Sqlite;
    use crate::sqlite::SqliteBindValue;
    use crate::sqlite::SqliteValue;
    use crate::test_helpers::connection;
    use serde_json::json;

    #[test]
    fn json_to_sql() {
        let conn = &mut connection();
        let value = json!(true);
        let res = diesel::select(value.into_sql::<Jsonb>().eq(sql("true")))
            .get_result::<bool>(conn)
            .unwrap();
        assert!(res);
    }

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

    // #[test]
    // fn some_json_from_sql() {
    //     let input_json = b"true";
    //     let output_json: serde_json::Value =
    //         FromSql::<sql_types::Json, Sqlite>::from_sql(SqliteValue::for_test(input_json))
    //             .unwrap();
    //     assert_eq!(output_json, serde_json::Value::Bool(true));
    // }

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
