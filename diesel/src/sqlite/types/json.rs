extern crate serde_json;

use crate::deserialize::{self, FromSql};
use crate::serialize::{self, IsNull, Output, ToSql};
use crate::sql_types;
use crate::sqlite::{Sqlite, SqliteValue};

const JSONB_NULL: u8 = 0x00;
const JSONB_TRUE: u8 = 0x01;
const JSONB_FALSE: u8 = 0x02;
const JSONB_INT: u8 = 0x03;
const JSONB_INT5: u8 = 0x04;
const JSONB_FLOAT: u8 = 0x05;
const JSONB_FLOAT5: u8 = 0x06;
const JSONB_TEXT: u8 = 0x07;
const JSONB_TEXTJ: u8 = 0x08;
const JSONB_TEXT5: u8 = 0x09;
const JSONB_TEXTRAW: u8 = 0x0A;
const JSONB_ARRAY: u8 = 0x0B;
const JSONB_OBJECT: u8 = 0x0C;

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

        if bytes.is_empty() {
            return Err("Empty blob cannot be decoded as JSONB".into());
        }

        // Read the JSONB value from the byte stream
        read_jsonb_value(&bytes)
    }
}

impl ToSql<sql_types::Jsonb, Sqlite> for serde_json::Value {
    fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, Sqlite>) -> serialize::Result {
        // Create a buffer to hold the binary JSONB encoding
        let mut buffer = Vec::new();

        // Write the JSON value into the buffer in JSONB format
        write_jsonb_value(self, &mut buffer)?;

        // Set the serialized binary data to the output
        out.set_value(buffer);

        Ok(IsNull::No)
    }
}

// Helper function to read a JSONB value from the byte stream
fn read_jsonb_value(bytes: &[u8]) -> deserialize::Result<serde_json::Value> {
    if bytes.is_empty() {
        return Err("Empty JSONB data".into());
    }

    // The first byte contains both the element type and potentially the payload size
    let first_byte = bytes[0];
    let element_type = first_byte & 0x0F;
    let payload_size_hint = (first_byte & 0xF0) >> 4;

    // Determine payload size and handle accordingly
    let (payload_size, payload_start) = match payload_size_hint {
        0x00..=0x0B => (payload_size_hint as usize, 1), // Payload size is encoded in the upper four bits directly
        0x0C => (bytes[1] as usize, 2),                 // 1 additional byte for payload size
        0x0D => (u16::from_be_bytes([bytes[1], bytes[2]]) as usize, 3), // 2 additional bytes for payload size
        0x0E => (
            u32::from_be_bytes([bytes[1], bytes[2], bytes[3], bytes[4]]) as usize,
            5,
        ), // 4 additional bytes
        0x0F => (
            u64::from_be_bytes([
                bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7], bytes[8],
            ]) as usize,
            9,
        ), // 8 additional bytes for payload size (unlikely in practice)
        _ => return Err("Invalid payload size hint".into()),
    };

    let remaining_bytes = &bytes[payload_start..];

    match element_type {
        JSONB_NULL => Ok(serde_json::Value::Null), // Null has no payload
        JSONB_TRUE => Ok(serde_json::Value::Bool(true)), // True has no payload
        JSONB_FALSE => Ok(serde_json::Value::Bool(false)), // False has no payload
        JSONB_INT => read_jsonb_int(remaining_bytes, payload_size),
        JSONB_INT5 => Err("INT5 is not supported in this implementation".into()), // INT5 not supported
        JSONB_FLOAT => read_jsonb_float(remaining_bytes, payload_size),
        JSONB_FLOAT5 => Err("FLOAT5 is not supported in this implementation".into()), // FLOAT5 not supported
        JSONB_TEXT => read_jsonb_text(remaining_bytes, payload_size),
        JSONB_TEXTJ => read_jsonb_text(remaining_bytes, payload_size), // Handle TEXTJ similarly to TEXT for now
        JSONB_TEXT5 => Err("TEXT5 is not supported in this implementation".into()), // TEXT5 not supported
        JSONB_TEXTRAW => read_jsonb_text(remaining_bytes, payload_size), // Handle TEXTRAW similarly to TEXT for now
        JSONB_ARRAY => read_jsonb_array(remaining_bytes, payload_size),
        JSONB_OBJECT => read_jsonb_object(remaining_bytes, payload_size),
        _ => Err(format!("Unsupported or reserved JSONB type: {}", element_type).into()),
    }
}

// Read a JSON integer in canonical format (INT)
fn read_jsonb_int(bytes: &[u8], _payload_size: usize) -> deserialize::Result<serde_json::Value> {
    let int_str = std::str::from_utf8(bytes).map_err(|_| "Invalid UTF-8 in JSONB integer")?;
    let int_value = int_str
        .parse::<i64>()
        .map_err(|_| "Failed to parse JSONB integer")?;
    Ok(serde_json::Value::Number(serde_json::Number::from(
        int_value,
    )))
}

// Read a JSON float in canonical format (FLOAT)
fn read_jsonb_float(bytes: &[u8], _payload_size: usize) -> deserialize::Result<serde_json::Value> {
    let float_str = std::str::from_utf8(bytes).map_err(|_| "Invalid UTF-8 in JSONB float")?;
    let float_value = float_str
        .parse::<f64>()
        .map_err(|_| "Failed to parse JSONB float")?;
    Ok(serde_json::Value::Number(
        serde_json::Number::from_f64(float_value).unwrap(),
    ))
}

// Read a JSON string
fn read_jsonb_text(bytes: &[u8], payload_size: usize) -> deserialize::Result<serde_json::Value> {
    let text_bytes = &bytes[..payload_size];
    let text = std::str::from_utf8(text_bytes).map_err(|_| "Invalid UTF-8 in JSONB string")?;
    Ok(serde_json::Value::String(text.to_string()))
}

// Read a JSON array
fn read_jsonb_array(bytes: &[u8], payload_size: usize) -> deserialize::Result<serde_json::Value> {
    let mut elements = Vec::new();
    let mut remaining_bytes = bytes;
    let mut total_read = 0;

    // Loop through the array elements and parse each one
    while total_read < payload_size {
        let element = read_jsonb_value(remaining_bytes)?;
        let element_size = remaining_bytes.len() - bytes.len();
        elements.push(element);
        remaining_bytes = &remaining_bytes[element_size..];
        total_read += element_size;
    }

    Ok(serde_json::Value::Array(elements))
}

// Read a JSON object
fn read_jsonb_object(bytes: &[u8], payload_size: usize) -> deserialize::Result<serde_json::Value> {
    let mut object = serde_json::Map::new();
    let mut remaining_bytes = bytes;
    let mut total_read = 0;

    // Loop through the object key-value pairs and parse each one
    while total_read < payload_size {
        let key_type = remaining_bytes[0] & 0x0F;

        // Ensure the key is a valid string type (TEXT, TEXTJ, TEXT5, TEXTRAW)
        if key_type != JSONB_TEXT
            && key_type != JSONB_TEXTJ
            && key_type != JSONB_TEXT5
            && key_type != JSONB_TEXTRAW
        {
            return Err(format!("Invalid JSONB object key type: {}", key_type).into());
        }

        // Read the key
        let key = read_jsonb_text(&remaining_bytes[1..], payload_size)?
            .as_str()
            .ok_or("Invalid object key in JSONB")?
            .to_string();
        let key_size = remaining_bytes.len() - bytes.len();
        remaining_bytes = &remaining_bytes[key_size + 1..];
        total_read += key_size + 1;

        // Read the value
        let value = read_jsonb_value(remaining_bytes)?;
        let value_size = remaining_bytes.len() - bytes.len();
        object.insert(key, value);
        remaining_bytes = &remaining_bytes[value_size..];
        total_read += value_size;
    }

    Ok(serde_json::Value::Object(object))
}

// Helper function to write a JSON value into a JSONB binary format
fn write_jsonb_value(value: &serde_json::Value, buffer: &mut Vec<u8>) -> serialize::Result {
    match value {
        serde_json::Value::Null => write_jsonb_null(buffer),
        serde_json::Value::Bool(b) => write_jsonb_bool(*b, buffer),
        serde_json::Value::Number(n) => write_jsonb_number(n, buffer),
        serde_json::Value::String(s) => write_jsonb_string(s, buffer),
        serde_json::Value::Array(arr) => write_jsonb_array(arr, buffer),
        serde_json::Value::Object(obj) => write_jsonb_object(obj, buffer),
    }
}

// Write a JSON null
fn write_jsonb_null(buffer: &mut Vec<u8>) -> serialize::Result {
    // Use the constant for null
    buffer.push(JSONB_NULL);
    Ok(IsNull::No)
}

// Write a JSON boolean
fn write_jsonb_bool(b: bool, buffer: &mut Vec<u8>) -> serialize::Result {
    // Use the constants for true and false
    let byte = if b { JSONB_TRUE } else { JSONB_FALSE };
    buffer.push(byte);
    Ok(IsNull::No)
}

// Write a JSON number (integers and floats)
fn write_jsonb_number(n: &serde_json::Number, buffer: &mut Vec<u8>) -> serialize::Result {
    if let Some(i) = n.as_i64() {
        // Write an integer (INT type)
        write_jsonb_int(i, buffer)
    } else if let Some(f) = n.as_f64() {
        // Write a float (FLOAT type)
        write_jsonb_float(f, buffer)
    } else {
        Err("Invalid number type".into())
    }
}

// Write an integer in JSONB format
fn write_jsonb_int(i: i64, buffer: &mut Vec<u8>) -> serialize::Result {
    // Use the constant for INT
    buffer.push(JSONB_INT);

    // Write the ASCII text representation of the integer as the payload
    buffer.extend_from_slice(i.to_string().as_bytes());

    Ok(IsNull::No)
}

// Write a floating-point number in JSONB format
fn write_jsonb_float(f: f64, buffer: &mut Vec<u8>) -> serialize::Result {
    // Use the constant for FLOAT
    buffer.push(JSONB_FLOAT);

    // Write the ASCII text representation of the float as the payload
    buffer.extend_from_slice(f.to_string().as_bytes());

    Ok(IsNull::No)
}

// Write a JSON string
fn write_jsonb_string(s: &str, buffer: &mut Vec<u8>) -> serialize::Result {
    // Use the constant for TEXT
    buffer.push(JSONB_TEXT);

    // Write the UTF-8 text of the string as the payload (no delimiters)
    buffer.extend_from_slice(s.as_bytes());

    Ok(IsNull::No)
}

// Write a JSON array
fn write_jsonb_array(arr: &[serde_json::Value], buffer: &mut Vec<u8>) -> serialize::Result {
    // Use the constant for ARRAY
    buffer.push(JSONB_ARRAY);

    // Recursively write each element of the array
    for element in arr {
        write_jsonb_value(element, buffer)?;
    }

    Ok(IsNull::No)
}

// Write a JSON object
fn write_jsonb_object(
    obj: &serde_json::Map<String, serde_json::Value>,
    buffer: &mut Vec<u8>,
) -> serialize::Result {
    // Use the constant for OBJECT
    buffer.push(JSONB_OBJECT);

    // Recursively write each key-value pair of the object
    for (key, value) in obj {
        // Write the key (which must be a string)
        write_jsonb_string(key, buffer)?;

        // Write the value
        write_jsonb_value(value, buffer)?;
    }

    Ok(IsNull::No)
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
