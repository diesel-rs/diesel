//! Support for JSON and JSONB values under SQLite.

use crate::deserialize::{self, FromSql};
use crate::serialize::{self, IsNull, Output, ToSql};
use crate::sql_types;
use crate::sqlite::{Sqlite, SqliteValue};

#[cfg(all(feature = "sqlite", feature = "serde_json"))]
impl FromSql<sql_types::Json, Sqlite> for serde_json::Value {
    fn from_sql(mut value: SqliteValue<'_, '_, '_>) -> deserialize::Result<Self> {
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
    fn from_sql(mut value: SqliteValue<'_, '_, '_>) -> deserialize::Result<Self> {
        use self::jsonb::*;

        let bytes = value.read_blob();

        if bytes.is_empty() {
            return Err("Empty blob cannot be decoded as JSONB".into());
        }

        // Read the JSONB value from the byte stream
        let (jsonb, _size) = read_jsonb_value(bytes)?;

        Ok(jsonb)
    }
}

#[cfg(all(feature = "sqlite", feature = "serde_json"))]
impl ToSql<sql_types::Jsonb, Sqlite> for serde_json::Value {
    fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, Sqlite>) -> serialize::Result {
        use self::jsonb::*;

        // Create a buffer to hold the binary JSONB encoding
        let mut buffer = Vec::new();

        // Write the JSON value into the buffer in JSONB format
        write_jsonb_value(self, &mut buffer)?;

        // Set the serialized binary data to the output
        out.set_value(buffer);

        Ok(IsNull::No)
    }
}

#[cfg(all(feature = "sqlite", feature = "serde_json"))]
mod jsonb {
    extern crate serde_json;

    use super::*;

    pub(super) const JSONB_NULL: u8 = 0x00;
    pub(super) const JSONB_TRUE: u8 = 0x01;
    pub(super) const JSONB_FALSE: u8 = 0x02;
    pub(super) const JSONB_INT: u8 = 0x03;
    pub(super) const JSONB_INT5: u8 = 0x04;
    pub(super) const JSONB_FLOAT: u8 = 0x05;
    pub(super) const JSONB_FLOAT5: u8 = 0x06;
    pub(super) const JSONB_TEXT: u8 = 0x07;
    pub(super) const JSONB_TEXTJ: u8 = 0x08;
    pub(super) const JSONB_TEXT5: u8 = 0x09;
    pub(super) const JSONB_TEXTRAW: u8 = 0x0A;
    pub(super) const JSONB_ARRAY: u8 = 0x0B;
    pub(super) const JSONB_OBJECT: u8 = 0x0C;

    // Helper function to read a JSONB value from the byte stream
    pub(super) fn read_jsonb_value(
        bytes: &[u8],
    ) -> deserialize::Result<(serde_json::Value, usize)> {
        if bytes.is_empty() {
            return Err("Empty JSONB data".into());
        }

        // The first byte contains both the element type and potentially the payload size
        let first_byte = bytes[0];
        let element_type = first_byte & 0x0F;
        let size_hint = (first_byte & 0xF0) >> 4;

        let (payload_size, header_size) = match size_hint {
            0x00..=0x0B => (size_hint as usize, 1), // Payload size is directly in the upper nibble
            0x0C => {
                if bytes.len() < 2 {
                    return Err("Invalid JSONB data: insufficient bytes for payload size".into());
                }
                (bytes[1] as usize, 2) // 1 additional byte for payload size
            }
            0x0D => {
                if bytes.len() < 3 {
                    return Err("Invalid JSONB data: insufficient bytes for payload size".into());
                }
                (u16::from_be_bytes([bytes[1], bytes[2]]) as usize, 3) // 2 additional bytes
            }
            0x0E => {
                if bytes.len() < 5 {
                    return Err("Invalid JSONB data: insufficient bytes for payload size".into());
                }
                (
                    u32::from_be_bytes([bytes[1], bytes[2], bytes[3], bytes[4]]) as usize,
                    5,
                ) // 4 additional bytes
            }
            0x0F => {
                if bytes.len() < 9 {
                    return Err("Invalid JSONB data: insufficient bytes for payload size".into());
                }
                (
                    usize::try_from(u64::from_be_bytes([
                        bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7],
                        bytes[8],
                    ]))
                    .map_err(Box::new)?,
                    9,
                ) // 8 additional bytes
            }
            _ => return Err("Invalid payload size hint".into()),
        };

        let total_size = header_size + payload_size;
        if bytes.len() < total_size {
            return Err(format!(
                "Invalid JSONB data: insufficient bytes for value of type {}, expected {} bytes, got {}",
                element_type,
                total_size,
                bytes.len()
            )
            .into());
        }

        let payload_bytes = &bytes[header_size..total_size];

        let value = match element_type {
            JSONB_NULL => Ok(serde_json::Value::Null),
            JSONB_TRUE => Ok(serde_json::Value::Bool(true)),
            JSONB_FALSE => Ok(serde_json::Value::Bool(false)),
            JSONB_INT => read_jsonb_int(payload_bytes, payload_size),
            JSONB_INT5 => Err("INT5 is not supported".into()),
            JSONB_FLOAT => read_jsonb_float(payload_bytes, payload_size),
            JSONB_FLOAT5 => Err("FLOAT5 is not supported".into()),
            JSONB_TEXT => read_jsonb_text(payload_bytes, payload_size),
            JSONB_TEXTJ => read_jsonb_textj(payload_bytes, payload_size),
            JSONB_TEXTRAW => Err("TEXTRAW is not supported".into()),
            JSONB_TEXT5 => Err("TEXT5 is not supported".into()),
            JSONB_ARRAY => read_jsonb_array(payload_bytes, payload_size),
            JSONB_OBJECT => read_jsonb_object(payload_bytes, payload_size),
            _ => Err(format!("Unsupported or reserved JSONB type: {element_type}").into()),
        }?;

        Ok((value, total_size))
    }

    // Read a JSON integer in canonical format (INT)
    pub(super) fn read_jsonb_int(
        bytes: &[u8],
        payload_size: usize,
    ) -> deserialize::Result<serde_json::Value> {
        // Ensure the bytes are at least as large as the payload size
        if bytes.len() < payload_size {
            return Err(format!(
                "Expected payload of size {}, but got {}",
                payload_size,
                bytes.len()
            )
            .into());
        }

        // Read only the number of bytes specified by the payload size
        let int_str = std::str::from_utf8(bytes).map_err(|_| "Invalid ASCII in JSONB integer")?;
        let int_value = serde_json::from_str(int_str)
            .map_err(|_| "Failed to parse JSONB")
            .and_then(|v: serde_json::Value| {
                v.is_i64()
                    .then_some(v)
                    .ok_or("Failed to parse JSONB integer")
            })?;

        Ok(int_value)
    }

    // Read a JSON float in canonical format (FLOAT)
    pub(super) fn read_jsonb_float(
        bytes: &[u8],
        payload_size: usize,
    ) -> deserialize::Result<serde_json::Value> {
        if bytes.len() < payload_size {
            return Err(format!(
                "Expected payload of size {}, but got {}",
                payload_size,
                bytes.len()
            )
            .into());
        }

        let float_str = std::str::from_utf8(bytes).map_err(|_| "Invalid UTF-8 in JSONB float")?;
        let float_value = serde_json::from_str(float_str)
            .map_err(|_| "Failed to parse JSONB")
            .and_then(|v: serde_json::Value| {
                v.is_f64()
                    .then_some(v)
                    .ok_or("Failed to parse JSONB number")
            })?;

        Ok(float_value)
    }

    // Read a JSON string
    pub(super) fn read_jsonb_text(
        bytes: &[u8],
        payload_size: usize,
    ) -> deserialize::Result<serde_json::Value> {
        if bytes.len() < payload_size {
            return Err(format!(
                "Expected payload of size {}, but got {}",
                payload_size,
                bytes.len()
            )
            .into());
        }

        let text = std::str::from_utf8(bytes).map_err(|_| "Invalid UTF-8 in JSONB string")?;
        Ok(serde_json::Value::String(text.to_string()))
    }

    pub(super) fn read_jsonb_textj(
        bytes: &[u8],
        payload_size: usize,
    ) -> deserialize::Result<serde_json::Value> {
        if bytes.len() < payload_size {
            return Err(format!(
                "Expected payload of size {}, but got {}",
                payload_size,
                bytes.len()
            )
            .into());
        }

        let text = std::str::from_utf8(bytes).map_err(|_| "Invalid UTF-8 in JSONB string")?;

        // Unescape JSON escape sequences (e.g., "\n", "\u0020")
        let unescaped_text = serde_json::from_str(&format!("\"{text}\""))
            .map_err(|_| "Failed to parse JSON-escaped text in TEXTJ")?;

        Ok(unescaped_text)
    }

    // Read a JSON array
    pub(super) fn read_jsonb_array(
        bytes: &[u8],
        payload_size: usize,
    ) -> deserialize::Result<serde_json::Value> {
        let mut elements = Vec::new();
        let mut total_read = 0;

        while total_read < payload_size {
            let (element, consumed) = read_jsonb_value(&bytes[total_read..payload_size])?;

            elements.push(element);
            total_read += consumed;
        }

        if total_read != payload_size {
            return Err("Array payload size mismatch".into());
        }

        Ok(serde_json::Value::Array(elements))
    }

    pub(super) fn read_jsonb_object(
        bytes: &[u8],
        payload_size: usize,
    ) -> deserialize::Result<serde_json::Value> {
        let mut object = serde_json::Map::new();
        let mut total_read = 0;

        while total_read < payload_size {
            // Read the key (must be a valid JSONB text type)
            let (key_value, key_consumed) = read_jsonb_value(&bytes[total_read..])?;
            let key_str = key_value
                .as_str()
                .ok_or("Invalid object key in JSONB, must be a string")?
                .to_string();
            total_read += key_consumed;

            // Read the value associated with the key
            let (value, value_consumed) = read_jsonb_value(&bytes[total_read..])?;
            object.insert(key_str, value);
            total_read += value_consumed;
        }

        // Ensure the total bytes read match the payload size
        if total_read != payload_size {
            return Err("Object payload size mismatch".into());
        }

        Ok(serde_json::Value::Object(object))
    }

    // Helper function to create the correct JsonbHeader based on the payload size
    pub(super) fn create_jsonb_header(
        element_type: u8,
        payload_size: usize,
    ) -> Result<Vec<u8>, String> {
        // Check if payload size exceeds the maximum allowed size
        if payload_size > 2_147_483_647 {
            return Err("Payload size exceeds the maximum allowed size of 2GB".into());
        }

        let header = if payload_size <= 0x0B {
            // Small payloads, 0 additional byte for size
            vec![((u8::try_from(payload_size).map_err(|e| e.to_string())?) << 4) | element_type]
        } else if payload_size <= 0xFF {
            // Medium payloads, 1 additional byte for size
            vec![
                (0x0C << 4) | element_type,
                u8::try_from(payload_size).map_err(|e| e.to_string())?,
            ]
        } else if payload_size <= 0xFFFF {
            let mut header = Vec::with_capacity(3);

            // Larger payloads, 2 additional bytes for size
            header.push((0x0D << 4) | element_type);
            header.extend_from_slice(
                &(u16::try_from(payload_size).map_err(|e| e.to_string())?).to_be_bytes(),
            );

            header
        } else {
            let mut header = Vec::with_capacity(5);

            // Very large payloads, 4 additional bytes for size (up to 2 GiB)
            header.push((0x0E << 4) | element_type);
            header.extend_from_slice(
                &(u32::try_from(payload_size).map_err(|e| e.to_string())?).to_be_bytes(),
            );

            header
        };

        Ok(header)
    }

    pub(super) fn write_jsonb_header(
        buffer: &mut Vec<u8>,
        element_type: u8,
        payload_size: usize,
    ) -> serialize::Result {
        // Create the header and append it to the buffer
        let header = create_jsonb_header(element_type, payload_size)?;
        buffer.extend(header);
        Ok(IsNull::No)
    }

    // Helper function to write a JSON value into a JSONB binary format
    pub(super) fn write_jsonb_value(
        value: &serde_json::Value,
        buffer: &mut Vec<u8>,
    ) -> serialize::Result {
        if value.is_null() {
            write_jsonb_null(buffer)
        } else if value.is_boolean() {
            write_jsonb_bool(value.as_bool().ok_or("Failed to read JSONB value")?, buffer)
        } else if value.is_number() {
            write_jsonb_number(value, buffer)
        } else if value.is_string() {
            write_jsonb_string(value.as_str().ok_or("Failed to read JSONB value")?, buffer)
        } else if value.is_array() {
            write_jsonb_array(
                value.as_array().ok_or("Failed to read JSONB value")?,
                buffer,
            )
        } else if value.is_object() {
            write_jsonb_object(
                value.as_object().ok_or("Failed to read JSONB value")?,
                buffer,
            )
        } else {
            Err("Unsupported JSONB value type".into())
        }
    }

    // Write a JSON null
    pub(super) fn write_jsonb_null(buffer: &mut Vec<u8>) -> serialize::Result {
        write_jsonb_header(buffer, JSONB_NULL, 0x0)?;
        Ok(IsNull::No)
    }

    // Write a JSON boolean
    pub(super) fn write_jsonb_bool(b: bool, buffer: &mut Vec<u8>) -> serialize::Result {
        // Use the constants for true and false
        write_jsonb_header(buffer, if b { JSONB_TRUE } else { JSONB_FALSE }, 0x0)?;
        Ok(IsNull::No)
    }

    // Write a JSON number (integers and floats)
    pub(super) fn write_jsonb_number(
        n: &serde_json::Value,
        buffer: &mut Vec<u8>,
    ) -> serialize::Result {
        if let Some(i) = n.as_i64() {
            // Write an integer (INT type)
            write_jsonb_int(i, buffer)
        } else if let Some(f) = n.as_f64() {
            // Write a float (FLOAT type)
            write_jsonb_float(f, buffer)
        } else {
            Err("Invalid JSONB number type".into())
        }
    }

    // Write an integer in JSONB format
    pub(super) fn write_jsonb_int(i: i64, buffer: &mut Vec<u8>) -> serialize::Result {
        let int_str = i.to_string();

        write_jsonb_header(buffer, JSONB_INT, int_str.len())?;

        // Write the ASCII text representation of the integer as the payload
        buffer.extend_from_slice(int_str.as_bytes());

        Ok(IsNull::No)
    }

    // Write a floating-point number in JSONB format
    pub(super) fn write_jsonb_float(f: f64, buffer: &mut Vec<u8>) -> serialize::Result {
        let float_str = f.to_string();

        write_jsonb_header(buffer, JSONB_FLOAT, float_str.len())?;

        // Write the ASCII text representation of the float as the payload
        buffer.extend_from_slice(float_str.as_bytes());

        Ok(IsNull::No)
    }

    pub(super) fn write_jsonb_string(s: &str, buffer: &mut Vec<u8>) -> serialize::Result {
        if s.chars().any(|c| c.is_control()) {
            // If the string contains control characters, treat it as TEXTJ (escaped JSON)
            write_jsonb_textj(s, buffer)
        } else {
            write_jsonb_header(buffer, JSONB_TEXT, s.len())?;
            // Write the UTF-8 text of the string as the payload (no delimiters)
            buffer.extend_from_slice(s.as_bytes());
            Ok(IsNull::No)
        }
    }

    pub(super) fn write_jsonb_textj(s: &str, buffer: &mut Vec<u8>) -> serialize::Result {
        // Escaping the string for JSON (e.g., \n, \uXXXX)
        let escaped_string = serde_json::to_string(&String::from(s))
            .map_err(|_| "Failed to serialize string for TEXTJ")?;

        // Remove the surrounding quotes from serde_json::to_string result
        let escaped_string = &escaped_string[1..escaped_string.len() - 1];

        // Write the header (JSONB_TEXTJ) and the length of the escaped string
        write_jsonb_header(buffer, JSONB_TEXTJ, escaped_string.len())?;

        // Write the escaped string as the payload
        buffer.extend_from_slice(escaped_string.as_bytes());

        Ok(IsNull::No)
    }

    // Write a JSON array
    pub(super) fn write_jsonb_array(
        arr: &[serde_json::Value],
        buffer: &mut Vec<u8>,
    ) -> serialize::Result {
        let mut tmp_buffer = Vec::new();

        // Recursively write each element of the array
        for element in arr {
            write_jsonb_value(element, &mut tmp_buffer)?;
        }

        write_jsonb_header(buffer, JSONB_ARRAY, tmp_buffer.len())?;

        buffer.extend_from_slice(&tmp_buffer);

        Ok(IsNull::No)
    }

    // Write a JSON object
    pub(super) fn write_jsonb_object(
        obj: &serde_json::Map<String, serde_json::Value>,
        buffer: &mut Vec<u8>,
    ) -> serialize::Result {
        let mut tmp_buffer = Vec::new();

        // Recursively write each key-value pair of the object
        for (key, value) in obj {
            // Write the key (which must be a string)
            write_jsonb_string(key, &mut tmp_buffer)?;

            // Write the value
            write_jsonb_value(value, &mut tmp_buffer)?;
        }

        write_jsonb_header(buffer, JSONB_OBJECT, tmp_buffer.len())?;

        buffer.extend_from_slice(&tmp_buffer);

        Ok(IsNull::No)
    }
}

#[cfg(test)]
#[cfg(all(feature = "sqlite", feature = "serde_json"))]
mod tests {
    use super::jsonb::*;
    use super::*;
    use crate::query_dsl::RunQueryDsl;
    use crate::test_helpers::connection;
    use crate::ExpressionMethods;
    use crate::{dsl::sql, IntoSql};
    use serde_json::{json, Value};
    use sql_types::{Json, Jsonb};

    #[diesel_test_helper::test]
    fn json_to_sql() {
        let conn = &mut connection();
        let res = diesel::select(json!(true).into_sql::<Json>().eq(&sql("json('true')")))
            .get_result::<bool>(conn)
            .unwrap();
        assert!(res);
    }

    #[diesel_test_helper::test]
    fn test_read_jsonb_null() {
        let data = vec![JSONB_NULL];
        let result = read_jsonb_value(&data).unwrap().0;
        assert_eq!(result, Value::Null);
    }

    #[diesel_test_helper::test]
    fn test_read_jsonb_true() {
        let data = vec![JSONB_TRUE];
        let result = read_jsonb_value(&data).unwrap().0;
        assert_eq!(result, Value::Bool(true));
    }

    #[diesel_test_helper::test]
    fn test_read_jsonb_false() {
        let data = vec![JSONB_FALSE];
        let result = read_jsonb_value(&data).unwrap().0;
        assert_eq!(result, Value::Bool(false));
    }

    #[diesel_test_helper::test]
    fn test_read_jsonb_int() {
        // JSONB_INT with payload "1"
        let mut data = Vec::new();
        data.extend(create_jsonb_header(JSONB_INT, 0x01).unwrap());
        data.push(b'1'); // Add the payload (integer "1")

        let result = read_jsonb_value(&data).unwrap().0;
        assert_eq!(result, json!(1));
    }

    #[diesel_test_helper::test]
    fn test_read_jsonb_float() {
        // JSONB_FLOAT with payload "1.5"
        let mut data = Vec::new();
        data.extend(create_jsonb_header(JSONB_FLOAT, 0x03).unwrap());
        data.extend_from_slice(b"1.5"); // Add the payload (float "1.5")

        let result = read_jsonb_value(&data).unwrap().0;
        assert_eq!(result, json!(1.5));
    }

    #[diesel_test_helper::test]
    fn test_read_jsonb_text() {
        // JSONB_TEXT with payload "foo"
        let mut data = Vec::new();
        data.extend(create_jsonb_header(JSONB_TEXT, 0x03).unwrap());
        data.extend_from_slice(b"foo"); // Add the payload (text "foo")

        let result = read_jsonb_value(&data).unwrap().0;
        assert_eq!(result, json!("foo"));
    }

    #[diesel_test_helper::test]
    fn test_read_jsonb_array() {
        // JSONB_ARRAY with two elements: 1 and true
        let mut data = Vec::new();
        data.extend(create_jsonb_header(JSONB_ARRAY, 0x03).unwrap()); // Array header

        // Element 1: integer "1"
        data.extend(create_jsonb_header(JSONB_INT, 0x01).unwrap());
        data.push(b'1');

        // Element 2: true
        data.extend(create_jsonb_header(JSONB_TRUE, 0x00).unwrap());

        let result = read_jsonb_value(&data).unwrap().0;
        assert_eq!(result, json!([1, true]));
    }

    #[diesel_test_helper::test]
    fn test_read_jsonb_object() {
        // JSONB_OBJECT with one key-value pair: "key": 42
        let mut data = Vec::new();
        data.extend(create_jsonb_header(JSONB_OBJECT, 0x07).unwrap()); // Object header

        // Key: "key"
        data.extend(create_jsonb_header(JSONB_TEXT, 0x03).unwrap());
        data.extend_from_slice(b"key"); // Add the key payload

        // Value: 42 (integer)
        data.extend(create_jsonb_header(JSONB_INT, 0x02).unwrap());
        data.extend_from_slice(b"42"); // Add the integer payload

        let result = read_jsonb_value(&data).unwrap().0;
        assert_eq!(result, json!({"key": 42}));
    }

    #[diesel_test_helper::test]
    fn test_read_jsonb_nested_object() {
        let mut data = Vec::new();

        data.extend(create_jsonb_header(JSONB_OBJECT, 42).unwrap());

        data.extend(create_jsonb_header(JSONB_TEXT, 9).unwrap());
        data.extend_from_slice(b"outer_key");

        data.extend(create_jsonb_header(JSONB_OBJECT, 13).unwrap());

        data.extend(create_jsonb_header(JSONB_TEXT, 9).unwrap());
        data.extend_from_slice(b"inner_key");

        data.extend(create_jsonb_header(JSONB_INT, 2).unwrap());
        data.extend_from_slice(b"42");

        data.extend(create_jsonb_header(JSONB_TEXT, 14).unwrap());
        data.extend_from_slice(b"additional_key");

        data.extend(create_jsonb_header(JSONB_TRUE, 0).unwrap());

        let result = read_jsonb_value(&data).unwrap().0;
        assert_eq!(
            result,
            json!({
                "additional_key": true,
                "outer_key": {
                    "inner_key": 42
                },
            })
        );
    }

    #[diesel_test_helper::test]
    fn test_write_jsonb_null() {
        let value = serde_json::Value::Null;
        let mut buffer = Vec::new();
        write_jsonb_value(&value, &mut buffer).unwrap();
        assert_eq!(buffer, vec![JSONB_NULL]);
    }

    #[diesel_test_helper::test]
    fn test_write_jsonb_true() {
        let value = serde_json::Value::Bool(true);
        let mut buffer = Vec::new();
        write_jsonb_value(&value, &mut buffer).unwrap();
        assert_eq!(buffer, vec![JSONB_TRUE]);
    }

    #[diesel_test_helper::test]
    fn test_write_jsonb_false() {
        let value = serde_json::Value::Bool(false);
        let mut buffer = Vec::new();
        write_jsonb_value(&value, &mut buffer).unwrap();
        assert_eq!(buffer, vec![JSONB_FALSE]);
    }

    #[diesel_test_helper::test]
    fn test_write_jsonb_int() {
        let value = serde_json::Value::Number(serde_json::Number::from(1));
        let mut buffer = Vec::new();
        write_jsonb_value(&value, &mut buffer).unwrap();

        let mut expected_buffer = Vec::new();
        expected_buffer.extend(create_jsonb_header(JSONB_INT, 0x01).unwrap());
        expected_buffer.push(b'1'); // Payload: integer "1"

        assert_eq!(buffer, expected_buffer);
    }

    #[diesel_test_helper::test]
    fn test_write_jsonb_float() {
        let value = serde_json::Value::Number(serde_json::Number::from_f64(1.5).unwrap());
        let mut buffer = Vec::new();
        write_jsonb_value(&value, &mut buffer).unwrap();

        let mut expected_buffer = Vec::new();
        expected_buffer.extend(create_jsonb_header(JSONB_FLOAT, 0x03).unwrap());
        expected_buffer.extend_from_slice(b"1.5"); // Payload: float "1.5"

        assert_eq!(buffer, expected_buffer);
    }

    #[diesel_test_helper::test]
    fn test_write_jsonb_text() {
        let mut buffer = Vec::new();
        let input_string = "hello";
        write_jsonb_string(input_string, &mut buffer).unwrap();

        let mut expected_buffer = Vec::new();
        expected_buffer.extend(create_jsonb_header(JSONB_TEXT, 0x05).unwrap());
        expected_buffer.extend_from_slice(b"hello");

        assert_eq!(buffer, expected_buffer);
    }

    #[diesel_test_helper::test]
    fn test_write_jsonb_textj() {
        let mut buffer = Vec::new();
        let input_string = "hello\nworld"; // Contains a newline, requires escaping
        write_jsonb_string(input_string, &mut buffer).unwrap();

        let mut expected_buffer = Vec::new();
        expected_buffer.extend(create_jsonb_header(JSONB_TEXTJ, 12).unwrap());
        expected_buffer.extend_from_slice(b"hello\\nworld");

        assert_eq!(buffer, expected_buffer);
    }

    #[diesel_test_helper::test]
    fn test_write_jsonb_array() {
        let value = json!([1, true]);
        let mut buffer = Vec::new();
        write_jsonb_value(&value, &mut buffer).unwrap();

        let mut expected_buffer = Vec::new();
        expected_buffer.extend(create_jsonb_header(JSONB_ARRAY, 0x03).unwrap()); // Array header
        expected_buffer.extend(create_jsonb_header(JSONB_INT, 0x01).unwrap()); // Integer header
        expected_buffer.push(b'1'); // Integer payload "1"
        expected_buffer.extend(create_jsonb_header(JSONB_TRUE, 0x00).unwrap()); // Boolean header for "true"

        assert_eq!(buffer, expected_buffer);
    }

    #[diesel_test_helper::test]
    fn test_write_jsonb_object() {
        let value = json!({"key": 42});
        let mut buffer = Vec::new();
        write_jsonb_value(&value, &mut buffer).unwrap();

        let mut expected = Vec::new();
        expected.extend(create_jsonb_header(JSONB_OBJECT, 7).unwrap());
        expected.extend(create_jsonb_header(JSONB_TEXT, 3).unwrap());
        expected.extend_from_slice(b"key");
        expected.extend(create_jsonb_header(JSONB_INT, 2).unwrap());
        expected.extend_from_slice(b"42");

        assert_eq!(buffer, expected,);
    }

    #[diesel_test_helper::test]
    fn jsonb_to_sql_bool() {
        let conn = &mut connection();
        let res = diesel::select(json!(true).into_sql::<Jsonb>().eq(&sql("jsonb('true')")))
            .get_result::<bool>(conn)
            .unwrap();
        assert!(res);
    }

    #[diesel_test_helper::test]
    fn jsonb_to_sql_null() {
        let conn = &mut connection();
        let res = diesel::select(json!(null).into_sql::<Jsonb>().eq(&sql("jsonb('null')")))
            .get_result::<bool>(conn)
            .unwrap();
        assert!(res);
    }

    #[diesel_test_helper::test]
    fn jsonb_to_sql_integer() {
        let conn = &mut connection();
        let res = diesel::select(json!(42).into_sql::<Jsonb>().eq(&sql("jsonb('42')")))
            .get_result::<bool>(conn)
            .unwrap();
        assert!(res);
    }

    #[diesel_test_helper::test]
    fn jsonb_to_sql_float() {
        let conn = &mut connection();
        let res = diesel::select(json!(42.23).into_sql::<Jsonb>().eq(&sql("jsonb('42.23')")))
            .get_result::<bool>(conn)
            .unwrap();
        assert!(res);
    }

    #[diesel_test_helper::test]
    fn jsonb_to_sql_text() {
        let conn = &mut connection();

        // Test for TEXT (simple string)
        let res = diesel::select(
            json!("hello")
                .into_sql::<Jsonb>()
                .eq(&sql("jsonb('\"hello\"')")),
        )
        .get_result::<bool>(conn)
        .unwrap();

        assert!(res);
    }

    #[diesel_test_helper::test]
    fn jsonb_to_sql_textj() {
        let conn = &mut connection();

        // Test for TEXTJ (JSON-escaped string, e.g., containing \n or \uXXXX)
        let res = diesel::select(
            json!("hello\nworld")
                .into_sql::<Jsonb>()
                .eq(&sql("jsonb('\"hello\\nworld\"')")), // The string is JSON-escaped
        )
        .get_result::<bool>(conn)
        .unwrap();

        assert!(res);
    }

    #[diesel_test_helper::test]
    fn jsonb_to_sql_array() {
        let conn = &mut connection();
        let res = diesel::select(
            json!([1, true, "foo"])
                .into_sql::<Jsonb>()
                .eq(&sql("jsonb('[1, true, \"foo\"]')")),
        )
        .get_result::<bool>(conn)
        .unwrap();
        assert!(res);
    }

    #[diesel_test_helper::test]
    fn jsonb_to_sql_object() {
        let conn = &mut connection();
        let res = diesel::select(
            json!({"key": "value"})
                .into_sql::<Jsonb>()
                .eq(&sql("jsonb('{\"key\": \"value\"}')")),
        )
        .get_result::<bool>(conn)
        .unwrap();
        assert!(res);
    }

    #[diesel_test_helper::test]
    fn jsonb_to_sql_object_in_object() {
        let conn = &mut connection();
        let json_value = json!({
            "outer_key": {
                "additional_key": true,
                "inner_key": {
                    "nested_key": 42
                },
            }
        });
        let res = diesel::select(json_value.into_sql::<Jsonb>().eq(&sql(
            r#"jsonb('{"outer_key": {"additional_key": true, "inner_key": {"nested_key": 42}}}')"#,
        )))
        .get_result::<bool>(conn)
        .unwrap();
        assert!(res);
    }

    #[diesel_test_helper::test]
    fn jsonb_to_sql_array_in_object() {
        let conn = &mut connection();
        let json_value = json!({
            "is_valid": false,
            "key": [1, 2, 3],
        });
        let res = diesel::select(
            json_value
                .into_sql::<Jsonb>()
                .eq(&sql(r#"jsonb('{"is_valid": false, "key": [1, 2, 3]}')"#)),
        )
        .get_result::<bool>(conn)
        .unwrap();
        assert!(res);
    }

    #[diesel_test_helper::test]
    fn jsonb_to_sql_object_in_array() {
        let conn = &mut connection();
        let json_value = json!([
            {
                "nested_key": "nested_value"
            },
            {
                "int_value": 99
            }
        ]);
        let res = diesel::select(json_value.into_sql::<Jsonb>().eq(&sql(
            r#"jsonb('[{"nested_key": "nested_value"}, {"int_value": 99}]')"#,
        )))
        .get_result::<bool>(conn)
        .unwrap();
        assert!(res);
    }

    #[diesel_test_helper::test]
    fn jsonb_from_sql_null() {
        let conn = &mut connection();
        let res = diesel::select(sql::<Jsonb>("jsonb('null')"))
            .get_result::<serde_json::Value>(conn)
            .unwrap();
        assert_eq!(res, serde_json::json!(null));
    }

    #[diesel_test_helper::test]
    fn jsonb_from_sql_true() {
        let conn = &mut connection();
        let res = diesel::select(sql::<Jsonb>("jsonb('true')"))
            .get_result::<serde_json::Value>(conn)
            .unwrap();
        assert_eq!(res, serde_json::json!(true));
    }

    #[diesel_test_helper::test]
    fn jsonb_from_sql_false() {
        let conn = &mut connection();
        let res = diesel::select(sql::<Jsonb>("jsonb('false')"))
            .get_result::<serde_json::Value>(conn)
            .unwrap();
        assert_eq!(res, serde_json::json!(false));
    }

    #[diesel_test_helper::test]
    fn jsonb_from_sql_int() {
        let conn = &mut connection();
        let res = diesel::select(sql::<Jsonb>("jsonb('42')"))
            .get_result::<serde_json::Value>(conn)
            .unwrap();
        assert_eq!(res, serde_json::json!(42));
    }

    #[diesel_test_helper::test]
    fn jsonb_from_sql_float() {
        let conn = &mut connection();
        let res = diesel::select(sql::<Jsonb>("jsonb('42.23')"))
            .get_result::<serde_json::Value>(conn)
            .unwrap();
        assert_eq!(res, serde_json::json!(42.23));
    }

    #[diesel_test_helper::test]
    fn jsonb_from_sql_object() {
        let conn = &mut connection();
        let res = diesel::select(sql::<Jsonb>("jsonb('{\"key\": \"value\"}')"))
            .get_result::<serde_json::Value>(conn)
            .unwrap();
        assert_eq!(res, serde_json::json!({"key": "value"}));
    }

    #[diesel_test_helper::test]
    fn jsonb_from_sql_array() {
        let conn = &mut connection();
        let res = diesel::select(sql::<Jsonb>("jsonb('[1, 2, 3]')"))
            .get_result::<serde_json::Value>(conn)
            .unwrap();
        assert_eq!(res, serde_json::json!([1, 2, 3]));
    }

    #[diesel_test_helper::test]
    fn jsonb_from_sql_nested_objects() {
        let conn = &mut connection();
        let res = diesel::select(sql::<Jsonb>("jsonb('{\"outer\": {\"inner\": 42}}')"))
            .get_result::<serde_json::Value>(conn)
            .unwrap();
        assert_eq!(res, serde_json::json!({"outer": {"inner": 42}}));
    }

    #[diesel_test_helper::test]
    fn jsonb_from_sql_nested_arrays() {
        let conn = &mut connection();
        let res = diesel::select(sql::<Jsonb>("jsonb('[[1, 2], [3, 4]]')"))
            .get_result::<serde_json::Value>(conn)
            .unwrap();
        assert_eq!(res, serde_json::json!([[1, 2], [3, 4]]));
    }

    #[diesel_test_helper::test]
    fn jsonb_from_sql_nested_arrays_in_objects() {
        let conn = &mut connection();
        let res = diesel::select(sql::<Jsonb>("jsonb('{\"array\": [1, 2, 3]}')"))
            .get_result::<serde_json::Value>(conn)
            .unwrap();
        assert_eq!(res, serde_json::json!({"array": [1, 2, 3]}));
    }

    #[diesel_test_helper::test]
    fn jsonb_from_sql_nested_objects_in_arrays() {
        let conn = &mut connection();
        let res = diesel::select(sql::<Jsonb>(
            "jsonb('[{\"key1\": \"value1\"}, {\"key2\": \"value2\"}]')",
        ))
        .get_result::<serde_json::Value>(conn)
        .unwrap();
        assert_eq!(
            res,
            serde_json::json!([{"key1": "value1"}, {"key2": "value2"}])
        );
    }

    #[diesel_test_helper::test]
    fn jsonb_from_sql_text() {
        let conn = &mut connection();
        let res = diesel::select(sql::<Jsonb>("jsonb('\"hello\"')"))
            .get_result::<serde_json::Value>(conn)
            .unwrap();
        assert_eq!(res, serde_json::json!("hello"));
    }

    #[diesel_test_helper::test]
    fn jsonb_from_sql_textj() {
        let conn = &mut connection();
        let res = diesel::select(sql::<Jsonb>("jsonb('\"hello\\nworld\"')"))
            .get_result::<serde_json::Value>(conn)
            .unwrap();
        assert_eq!(res, serde_json::json!("hello\nworld"));
    }

    #[diesel_test_helper::test]
    fn bad_json_from_sql() {
        let conn = &mut connection();
        let res = diesel::select(json!(true).into_sql::<Json>().eq(&sql("json('boom')")))
            .get_result::<bool>(conn);
        assert_eq!(res.unwrap_err().to_string(), "malformed JSON");
    }

    #[diesel_test_helper::test]
    fn bad_jsonb_from_sql() {
        let conn = &mut connection();
        let res = diesel::select(json!(true).into_sql::<Jsonb>().eq(&sql("jsonb('boom')")))
            .get_result::<bool>(conn);
        assert_eq!(res.unwrap_err().to_string(), "malformed JSON");
    }

    #[diesel_test_helper::test]
    fn no_json_from_sql() {
        let uuid: Result<serde_json::Value, _> = FromSql::<Json, Sqlite>::from_nullable_sql(None);
        assert_eq!(
            uuid.unwrap_err().to_string(),
            "Unexpected null for non-null column"
        );
    }

    #[diesel_test_helper::test]
    fn no_jsonb_from_sql() {
        let uuid: Result<serde_json::Value, _> = FromSql::<Jsonb, Sqlite>::from_nullable_sql(None);
        assert_eq!(
            uuid.unwrap_err().to_string(),
            "Unexpected null for non-null column"
        );
    }
}
