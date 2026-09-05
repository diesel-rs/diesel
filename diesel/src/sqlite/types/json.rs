//! Support for JSON and JSONB values under SQLite.

use crate::deserialize::{self, FromSql};
use crate::serialize::{self, IsNull, Output, ToSql};
use crate::sql_types;
use crate::sqlite::{Sqlite, SqliteValue};
use alloc::boxed::Box;
use alloc::string::ToString;
use alloc::vec::Vec;

#[cfg(all(feature = "__sqlite-shared", feature = "serde_json"))]
impl FromSql<sql_types::Json, Sqlite> for serde_json::Value {
    fn from_sql(mut value: SqliteValue<'_, '_, '_>) -> deserialize::Result<Self> {
        serde_json::from_str(value.read_text()).map_err(|_| "Invalid Json".into())
    }
}

#[cfg(all(feature = "__sqlite-shared", feature = "serde_json"))]
impl ToSql<sql_types::Json, Sqlite> for serde_json::Value {
    fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, Sqlite>) -> serialize::Result {
        out.set_value(serde_json::to_string(self)?);
        Ok(IsNull::No)
    }
}

#[cfg(all(feature = "__sqlite-shared", feature = "serde_json"))]
impl FromSql<sql_types::Jsonb, Sqlite> for serde_json::Value {
    fn from_sql(mut value: SqliteValue<'_, '_, '_>) -> deserialize::Result<Self> {
        use self::jsonb::*;

        let bytes = value.read_blob();

        if bytes.is_empty() {
            return Err("Empty blob cannot be decoded as JSONB".into());
        }

        // Read the JSONB value from the byte stream
        let (jsonb, size) = read_jsonb_value(bytes)?;
        if size == bytes.len() {
            Ok(jsonb)
        } else {
            Err("Payload contained more bytes than the encoded JSONB".into())
        }
    }
}

#[cfg(all(feature = "__sqlite-shared", feature = "serde_json"))]
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

#[cfg(all(feature = "__sqlite-shared", feature = "serde_json"))]
mod jsonb {
    extern crate serde_json;

    use alloc::vec;
    use core::error::Error;

    use super::*;

    type JsonbResult<T> = core::result::Result<T, Box<dyn Error + Send + Sync>>;

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

    #[derive(Debug)]
    struct JsonbHeader {
        element_type: u8,
        payload_size: usize,
        header_size: usize,
        total_size: usize,
    }

    // Helper function to read a JSONB value from the byte stream
    #[allow(unsafe_code)]
    pub(super) fn read_jsonb_value(
        bytes: &[u8],
    ) -> deserialize::Result<(serde_json::Value, usize)> {
        if bytes.is_empty() {
            return Err("Empty JSONB data".into());
        }
        let (global_header, mut global_value) = read_header_and_value(bytes)?;

        if global_value.is_array() || global_value.is_object() {
            // we need to use pointers here, as the borrow checker does not understand
            // that we only modify the last value in this stack. Given that we cannot
            // invalidate any pointer that's up in the stack
            let mut stack = vec![(
                &mut global_value as *mut serde_json::Value,
                global_header.payload_size,
            )];
            let mut payload = &bytes[global_header.header_size..];
            let mut total_read = 0;
            // we must use a loop based decoding approach here instead of the much simpler recursive implementation
            // as we otherwise run into stack overflows for deeply nested objects/arrays

            while total_read < global_header.payload_size {
                let Some((value, _size)) = stack.last().copied() else {
                    break;
                };
                let value = unsafe {
                    // SAFETY: The pointer was initialized before
                    // We we cannot invalidate the underlying object
                    &mut *value
                };

                if let serde_json::Value::Array(array) = value {
                    let (header, value) = read_header_and_value(payload)?;

                    array.push(value);
                    let last_ref = array.last_mut().expect("Pushed above");
                    let payload_size = if last_ref.is_object() || last_ref.is_array() {
                        stack.push((last_ref as *mut _, total_read + header.total_size));
                        header.header_size
                    } else {
                        header.total_size
                    };
                    total_read += payload_size;
                    if payload.len() > payload_size {
                        payload = &payload[payload_size..];
                    } else {
                        for (_, v) in stack {
                            if v != total_read {
                                return Err("Invalid size of payload declared".into());
                            }
                        }
                        break;
                    }
                } else if let serde_json::Value::Object(object) = value {
                    //       while total_read < payload_size {
                    let (key_header, key) = read_header_and_value(payload)?;
                    total_read += key_header.total_size;
                    let serde_json::Value::String(key) = key else {
                        return Err("Expected a string as object key".into());
                    };
                    if payload.len() > key_header.total_size {
                        payload = &payload[key_header.total_size..];
                    } else {
                        return Err("No value found for object".into());
                    }
                    let (value_header, value) = read_header_and_value(payload)?;
                    object.insert(key.clone(), value);
                    let last_ref = object.get_mut(&key).expect("We inserted it above");
                    let payload_size = if last_ref.is_object() || last_ref.is_array() {
                        stack.push((last_ref as *mut _, total_read + value_header.total_size));
                        value_header.header_size
                    } else {
                        value_header.total_size
                    };
                    total_read += payload_size;
                    if payload.len() > payload_size {
                        payload = &payload[payload_size..];
                    } else {
                        for (_, v) in stack {
                            if v != total_read {
                                return Err("Invalid size of payload declared".into());
                            }
                        }
                        break;
                    }
                } else {
                    unreachable!()
                }

                while let Some(v) = stack.last().map(|(_, v)| *v) {
                    if v > total_read {
                        break;
                    } else if v == total_read {
                        stack.pop();
                    } else {
                        return Err("Invalid size of payload declared".into());
                    }
                }
            }
        }
        Ok((global_value, global_header.total_size))
    }

    // This function decodes the jsonb header
    // and the value for non-composite values. For composite values like array and object
    // we only decode the "value header" and leave decoding
    // the actual child values to future calls
    fn read_header_and_value(
        bytes: &[u8],
    ) -> deserialize::Result<(JsonbHeader, serde_json::Value)> {
        let header = read_jsonb_value_header(bytes)?;
        let payload_bytes = &bytes[header.header_size..header.total_size];
        let value = match header.element_type {
            JSONB_NULL => Ok(serde_json::Value::Null),
            JSONB_TRUE => Ok(serde_json::Value::Bool(true)),
            JSONB_FALSE => Ok(serde_json::Value::Bool(false)),
            JSONB_INT => read_jsonb_int(payload_bytes, header.payload_size),
            JSONB_INT5 => Err("INT5 is not supported".into()),
            JSONB_FLOAT => read_jsonb_float(payload_bytes, header.payload_size),
            JSONB_FLOAT5 => Err("FLOAT5 is not supported".into()),
            JSONB_TEXT => read_jsonb_text(payload_bytes, header.payload_size),
            JSONB_TEXTJ => read_jsonb_textj(payload_bytes, header.payload_size),
            JSONB_TEXTRAW => read_jsonb_text(payload_bytes, header.payload_size),
            JSONB_TEXT5 => Err("TEXT5 is not supported".into()),
            JSONB_ARRAY => Ok(serde_json::Value::Array(alloc::vec::Vec::new())),
            JSONB_OBJECT => Ok(serde_json::Value::Object(serde_json::Map::new())),
            _ => Err(alloc::format!(
                "Unsupported or reserved JSONB type: {}",
                header.element_type
            )
            .into()),
        }?;
        Ok((header, value))
    }

    fn read_jsonb_value_header(bytes: &[u8]) -> deserialize::Result<JsonbHeader> {
        let first_byte = bytes[0];
        let element_type = first_byte & 0x0F;
        let size_hint = (first_byte & 0xF0) >> 4;
        let (payload_size, header_size): (usize, usize) = match size_hint {
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
        let total_size = header_size
            .checked_add(payload_size)
            .ok_or("The provided payload size overflows usize")?;
        if bytes.len() < total_size {
            return Err(alloc::format!(
                "Invalid JSONB data: insufficient bytes for value of type {}, expected {} bytes, got {}",
                element_type,
                total_size,
                bytes.len()
            )
            .into());
        }

        Ok(JsonbHeader {
            element_type,
            payload_size,
            header_size,
            total_size,
        })
    }

    // Read a JSON integer in canonical format (INT)
    pub(super) fn read_jsonb_int(
        bytes: &[u8],
        payload_size: usize,
    ) -> deserialize::Result<serde_json::Value> {
        // Ensure the bytes are at least as large as the payload size
        if bytes.len() < payload_size {
            return Err(alloc::format!(
                "Expected payload of size {}, but got {}",
                payload_size,
                bytes.len()
            )
            .into());
        }

        // Read only the number of bytes specified by the payload size
        let int_str = core::str::from_utf8(bytes).map_err(|_| "Invalid ASCII in JSONB integer")?;
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
            return Err(alloc::format!(
                "Expected payload of size {}, but got {}",
                payload_size,
                bytes.len()
            )
            .into());
        }

        let float_str = core::str::from_utf8(bytes).map_err(|_| "Invalid UTF-8 in JSONB float")?;
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
            return Err(alloc::format!(
                "Expected payload of size {}, but got {}",
                payload_size,
                bytes.len()
            )
            .into());
        }

        let text = core::str::from_utf8(bytes).map_err(|_| "Invalid UTF-8 in JSONB string")?;
        Ok(serde_json::Value::String(text.to_string()))
    }

    pub(super) fn read_jsonb_textj(
        bytes: &[u8],
        payload_size: usize,
    ) -> deserialize::Result<serde_json::Value> {
        if bytes.len() < payload_size {
            return Err(alloc::format!(
                "Expected payload of size {}, but got {}",
                payload_size,
                bytes.len()
            )
            .into());
        }

        let text = core::str::from_utf8(bytes).map_err(|_| "Invalid UTF-8 in JSONB string")?;

        // Unescape JSON escape sequences (e.g., "\n", "\u0020")
        let unescaped_text = serde_json::from_str(&alloc::format!("\"{text}\""))
            .map_err(|_| "Failed to parse JSON-escaped text in TEXTJ")?;

        Ok(unescaped_text)
    }

    fn jsonb_header_size(payload_size: usize) -> JsonbResult<usize> {
        if payload_size > 2_147_483_647 {
            Err("Payload size exceeds the maximum allowed size of 2GB".into())
        } else if payload_size <= 0x0B {
            Ok(1)
        } else if payload_size <= 0xFF {
            Ok(2)
        } else if payload_size <= 0xFFFF {
            Ok(3)
        } else {
            Ok(5)
        }
    }

    pub(super) fn write_jsonb_header(
        buffer: &mut Vec<u8>,
        element_type: u8,
        payload_size: usize,
    ) -> serialize::Result {
        jsonb_header_size(payload_size)?;

        if payload_size <= 0x0B {
            // Small payloads, 0 additional byte for size
            buffer.push(
                ((u8::try_from(payload_size).map_err(|e| e.to_string())?) << 4) | element_type,
            );
        } else if payload_size <= 0xFF {
            // Medium payloads, 1 additional byte for size
            buffer.extend_from_slice(&[
                (0x0C << 4) | element_type,
                u8::try_from(payload_size).map_err(|e| e.to_string())?,
            ]);
        } else if payload_size <= 0xFFFF {
            // Larger payloads, 2 additional bytes for size
            buffer.push((0x0D << 4) | element_type);
            buffer.extend_from_slice(
                &(u16::try_from(payload_size).map_err(|e| e.to_string())?).to_be_bytes(),
            );
        } else {
            // Very large payloads, 4 additional bytes for size (up to 2 GiB)
            buffer.push((0x0E << 4) | element_type);
            buffer.extend_from_slice(
                &(u32::try_from(payload_size).map_err(|e| e.to_string())?).to_be_bytes(),
            );
        };

        Ok(IsNull::No)
    }

    #[inline]
    fn u64_len(n: u64) -> usize {
        if n == 0 { 1 } else { (n.ilog10() + 1) as usize }
    }

    #[inline]
    fn i64_len(n: i64) -> usize {
        if n >= 0 {
            u64_len(n.unsigned_abs())
        } else {
            1 + u64_len(n.unsigned_abs())
        }
    }

    fn jsonb_number_size(value: &serde_json::Number) -> (u8, usize) {
        if let Some(i) = value.as_i64() {
            (JSONB_INT, i64_len(i))
        } else if let Some(u) = value.as_u64() {
            (JSONB_INT, u64_len(u))
        } else {
            let value_str = value.to_string();
            let element_type = if value_str
                .char_indices()
                .any(|(idx, c)| !(c.is_ascii_digit() || (idx == 0 && (c == '-' || c == '+'))))
            {
                JSONB_FLOAT
            } else {
                JSONB_INT
            };
            (element_type, value_str.len())
        }
    }

    fn jsonb_string_size(value: &str) -> JsonbResult<(u8, usize)> {
        if value.chars().any(|c| c.is_control()) {
            let mut escaped_len = 0usize;
            for c in value.chars() {
                let char_len = match c {
                    '"' | '\\' | '\x08' | '\x0C' | '\n' | '\r' | '\t' => 2,
                    c if c.is_control() => 6,
                    _ => c.len_utf8(),
                };
                escaped_len = escaped_len
                    .checked_add(char_len)
                    .ok_or("The encoded JSONB size overflows usize")?;
            }
            Ok((JSONB_TEXTJ, escaped_len))
        } else {
            Ok((JSONB_TEXT, value.len()))
        }
    }

    fn jsonb_scalar_encoded_size(value: &serde_json::Value) -> JsonbResult<usize> {
        let payload_size = match value {
            serde_json::Value::Null | serde_json::Value::Bool(_) => 0,
            serde_json::Value::Number(n) => jsonb_number_size(n).1,
            serde_json::Value::String(s) => jsonb_string_size(s)?.1,
            serde_json::Value::Array(_) | serde_json::Value::Object(_) => {
                unreachable!("Not a scalar")
            }
        };
        let header_len = jsonb_header_size(payload_size)?;
        header_len
            .checked_add(payload_size)
            .ok_or_else(|| "The encoded JSONB size overflows usize".into())
    }

    fn jsonb_string_encoded_size(value: &str) -> JsonbResult<usize> {
        let (_, payload_size) = jsonb_string_size(value)?;
        let header_len = jsonb_header_size(payload_size)?;
        header_len
            .checked_add(payload_size)
            .ok_or_else(|| "The encoded JSONB size overflows usize".into())
    }

    #[inline]
    fn write_u64_bytes(buffer: &mut Vec<u8>, mut n: u64) {
        if n == 0 {
            buffer.push(b'0');
            return;
        }
        let mut buf = [0u8; 20];
        let mut i = 20;
        while n > 0 {
            i -= 1;
            buf[i] = b'0' + (n % 10) as u8;
            n /= 10;
        }
        buffer.extend_from_slice(&buf[i..]);
    }

    #[inline]
    fn write_i64_bytes(buffer: &mut Vec<u8>, n: i64) {
        if n >= 0 {
            write_u64_bytes(buffer, n.unsigned_abs());
        } else {
            buffer.push(b'-');
            write_u64_bytes(buffer, n.unsigned_abs());
        }
    }

    fn write_jsonb_string(s: &str, buffer: &mut Vec<u8>) -> serialize::Result {
        if s.chars().any(|c| c.is_control()) {
            let escaped =
                serde_json::to_string(s).map_err(|_| "Failed to serialize string for TEXTJ")?;
            let payload = &escaped[1..escaped.len() - 1];
            write_jsonb_header(buffer, JSONB_TEXTJ, payload.len())?;
            buffer.extend_from_slice(payload.as_bytes());
        } else {
            write_jsonb_header(buffer, JSONB_TEXT, s.len())?;
            buffer.extend_from_slice(s.as_bytes());
        }
        Ok(IsNull::No)
    }

    fn write_jsonb_scalar(value: &serde_json::Value, buffer: &mut Vec<u8>) -> serialize::Result {
        match value {
            serde_json::Value::Null => {
                write_jsonb_header(buffer, JSONB_NULL, 0)?;
            }
            serde_json::Value::Bool(b) => {
                write_jsonb_header(buffer, if *b { JSONB_TRUE } else { JSONB_FALSE }, 0)?;
            }
            serde_json::Value::Number(n) => {
                if let Some(i) = n.as_i64() {
                    let len = i64_len(i);
                    write_jsonb_header(buffer, JSONB_INT, len)?;
                    write_i64_bytes(buffer, i);
                } else if let Some(u) = n.as_u64() {
                    let len = u64_len(u);
                    write_jsonb_header(buffer, JSONB_INT, len)?;
                    write_u64_bytes(buffer, u);
                } else {
                    let n_str = n.to_string();
                    let element_type = if n_str.char_indices().any(|(idx, c)| {
                        !(c.is_ascii_digit() || (idx == 0 && (c == '-' || c == '+')))
                    }) {
                        JSONB_FLOAT
                    } else {
                        JSONB_INT
                    };
                    write_jsonb_header(buffer, element_type, n_str.len())?;
                    buffer.extend_from_slice(n_str.as_bytes());
                }
            }
            serde_json::Value::String(s) => {
                write_jsonb_string(s, buffer)?;
            }
            serde_json::Value::Array(_) | serde_json::Value::Object(_) => {
                unreachable!("Not a scalar")
            }
        }
        Ok(IsNull::No)
    }

    enum ContainerIter<'a> {
        Array(core::slice::Iter<'a, serde_json::Value>),
        Object(serde_json::map::Iter<'a>),
    }

    struct SizeFrame<'a> {
        composite_idx: usize,
        iter: ContainerIter<'a>,
        running_payload_size: usize,
    }

    // Helper function to write a JSON value into a JSONB binary format
    pub(super) fn write_jsonb_value(
        value: &serde_json::Value,
        buffer: &mut Vec<u8>,
    ) -> serialize::Result {
        let (root_elem_type, root_iter) = match value {
            serde_json::Value::Array(values) => (JSONB_ARRAY, ContainerIter::Array(values.iter())),
            serde_json::Value::Object(object) => {
                (JSONB_OBJECT, ContainerIter::Object(object.iter()))
            }
            scalar => {
                let total_size = jsonb_scalar_encoded_size(scalar)?;
                buffer
                    .try_reserve(total_size)
                    .map_err(|error| error.to_string())?;
                write_jsonb_scalar(scalar, buffer)?;
                return Ok(IsNull::No);
            }
        };

        // Pass 1: Calculate composite container payload sizes using stack frames bounded
        // by the nesting depth. Empty composites have size 0 and do not require entries in
        // composite_sizes, ensuring auxiliary memory for wide shallow inputs (such as [[], [], ...])
        // remains O(depth) rather than O(total composites).
        let mut composite_sizes = vec![0];
        let mut stack = vec![SizeFrame {
            composite_idx: 0,
            iter: root_iter,
            running_payload_size: 0,
        }];
        let mut total_size = 0;

        while let Some(frame) = stack.last_mut() {
            match &mut frame.iter {
                ContainerIter::Array(iter) => match iter.next() {
                    Some(serde_json::Value::Array(nested_values)) => {
                        if nested_values.is_empty() {
                            let sz = jsonb_header_size(0)?;
                            frame.running_payload_size = frame
                                .running_payload_size
                                .checked_add(sz)
                                .ok_or("The encoded JSONB size overflows usize")?;
                        } else {
                            let composite_idx = composite_sizes.len();
                            composite_sizes.push(0);
                            stack.push(SizeFrame {
                                composite_idx,
                                iter: ContainerIter::Array(nested_values.iter()),
                                running_payload_size: 0,
                            });
                        }
                    }
                    Some(serde_json::Value::Object(nested_object)) => {
                        if nested_object.is_empty() {
                            let sz = jsonb_header_size(0)?;
                            frame.running_payload_size = frame
                                .running_payload_size
                                .checked_add(sz)
                                .ok_or("The encoded JSONB size overflows usize")?;
                        } else {
                            let composite_idx = composite_sizes.len();
                            composite_sizes.push(0);
                            stack.push(SizeFrame {
                                composite_idx,
                                iter: ContainerIter::Object(nested_object.iter()),
                                running_payload_size: 0,
                            });
                        }
                    }
                    Some(scalar) => {
                        let sz = jsonb_scalar_encoded_size(scalar)?;
                        frame.running_payload_size = frame
                            .running_payload_size
                            .checked_add(sz)
                            .ok_or("The encoded JSONB size overflows usize")?;
                    }
                    None => {
                        let finished = stack.pop().expect("frame exists");
                        composite_sizes[finished.composite_idx] = finished.running_payload_size;
                        let encoded_sz = jsonb_header_size(finished.running_payload_size)?
                            .checked_add(finished.running_payload_size)
                            .ok_or("The encoded JSONB size overflows usize")?;

                        if let Some(parent) = stack.last_mut() {
                            parent.running_payload_size = parent
                                .running_payload_size
                                .checked_add(encoded_sz)
                                .ok_or("The encoded JSONB size overflows usize")?;
                        } else {
                            total_size = encoded_sz;
                            break;
                        }
                    }
                },
                ContainerIter::Object(iter) => match iter.next() {
                    Some((key, val)) => {
                        let key_sz = jsonb_string_encoded_size(key)?;
                        frame.running_payload_size = frame
                            .running_payload_size
                            .checked_add(key_sz)
                            .ok_or("The encoded JSONB size overflows usize")?;

                        match val {
                            serde_json::Value::Array(nested_values) => {
                                if nested_values.is_empty() {
                                    let sz = jsonb_header_size(0)?;
                                    frame.running_payload_size = frame
                                        .running_payload_size
                                        .checked_add(sz)
                                        .ok_or("The encoded JSONB size overflows usize")?;
                                } else {
                                    let composite_idx = composite_sizes.len();
                                    composite_sizes.push(0);
                                    stack.push(SizeFrame {
                                        composite_idx,
                                        iter: ContainerIter::Array(nested_values.iter()),
                                        running_payload_size: 0,
                                    });
                                }
                            }
                            serde_json::Value::Object(nested_object) => {
                                if nested_object.is_empty() {
                                    let sz = jsonb_header_size(0)?;
                                    frame.running_payload_size = frame
                                        .running_payload_size
                                        .checked_add(sz)
                                        .ok_or("The encoded JSONB size overflows usize")?;
                                } else {
                                    let composite_idx = composite_sizes.len();
                                    composite_sizes.push(0);
                                    stack.push(SizeFrame {
                                        composite_idx,
                                        iter: ContainerIter::Object(nested_object.iter()),
                                        running_payload_size: 0,
                                    });
                                }
                            }
                            scalar => {
                                let val_sz = jsonb_scalar_encoded_size(scalar)?;
                                frame.running_payload_size = frame
                                    .running_payload_size
                                    .checked_add(val_sz)
                                    .ok_or("The encoded JSONB size overflows usize")?;
                            }
                        }
                    }
                    None => {
                        let finished = stack.pop().expect("frame exists");
                        composite_sizes[finished.composite_idx] = finished.running_payload_size;
                        let encoded_sz = jsonb_header_size(finished.running_payload_size)?
                            .checked_add(finished.running_payload_size)
                            .ok_or("The encoded JSONB size overflows usize")?;

                        if let Some(parent) = stack.last_mut() {
                            parent.running_payload_size = parent
                                .running_payload_size
                                .checked_add(encoded_sz)
                                .ok_or("The encoded JSONB size overflows usize")?;
                        } else {
                            total_size = encoded_sz;
                            break;
                        }
                    }
                },
            }
        }

        buffer
            .try_reserve(total_size)
            .map_err(|error| error.to_string())?;

        // Pass 2: Traverse the borrowed Value again in identical order, emitting precomputed
        // composite headers and scalar payloads directly into the pre-reserved destination.
        let mut composite_idx = 0;
        let payload_size = composite_sizes[composite_idx];
        composite_idx += 1;
        write_jsonb_header(buffer, root_elem_type, payload_size)?;

        let mut write_stack = vec![match value {
            serde_json::Value::Array(values) => ContainerIter::Array(values.iter()),
            serde_json::Value::Object(object) => ContainerIter::Object(object.iter()),
            _ => unreachable!(),
        }];

        while let Some(iter) = write_stack.last_mut() {
            match iter {
                ContainerIter::Array(it) => match it.next() {
                    Some(serde_json::Value::Array(nested_values)) => {
                        if nested_values.is_empty() {
                            write_jsonb_header(buffer, JSONB_ARRAY, 0)?;
                        } else {
                            let payload_size = composite_sizes[composite_idx];
                            composite_idx += 1;
                            write_jsonb_header(buffer, JSONB_ARRAY, payload_size)?;
                            write_stack.push(ContainerIter::Array(nested_values.iter()));
                        }
                    }
                    Some(serde_json::Value::Object(nested_object)) => {
                        if nested_object.is_empty() {
                            write_jsonb_header(buffer, JSONB_OBJECT, 0)?;
                        } else {
                            let payload_size = composite_sizes[composite_idx];
                            composite_idx += 1;
                            write_jsonb_header(buffer, JSONB_OBJECT, payload_size)?;
                            write_stack.push(ContainerIter::Object(nested_object.iter()));
                        }
                    }
                    Some(scalar) => {
                        write_jsonb_scalar(scalar, buffer)?;
                    }
                    None => {
                        write_stack.pop();
                    }
                },
                ContainerIter::Object(it) => match it.next() {
                    Some((key, val)) => {
                        write_jsonb_string(key, buffer)?;
                        match val {
                            serde_json::Value::Array(nested_values) => {
                                if nested_values.is_empty() {
                                    write_jsonb_header(buffer, JSONB_ARRAY, 0)?;
                                } else {
                                    let payload_size = composite_sizes[composite_idx];
                                    composite_idx += 1;
                                    write_jsonb_header(buffer, JSONB_ARRAY, payload_size)?;
                                    write_stack.push(ContainerIter::Array(nested_values.iter()));
                                }
                            }
                            serde_json::Value::Object(nested_object) => {
                                if nested_object.is_empty() {
                                    write_jsonb_header(buffer, JSONB_OBJECT, 0)?;
                                } else {
                                    let payload_size = composite_sizes[composite_idx];
                                    composite_idx += 1;
                                    write_jsonb_header(buffer, JSONB_OBJECT, payload_size)?;
                                    write_stack.push(ContainerIter::Object(nested_object.iter()));
                                }
                            }
                            scalar => {
                                write_jsonb_scalar(scalar, buffer)?;
                            }
                        }
                    }
                    None => {
                        write_stack.pop();
                    }
                },
            }
        }

        Ok(IsNull::No)
    }
}

#[cfg(test)]
#[cfg(all(feature = "__sqlite-shared", feature = "serde_json"))]
mod tests {
    use super::jsonb::*;
    use super::*;
    #[cfg(not(miri))] // ffi call
    use crate::ExpressionMethods;
    #[cfg(not(miri))] // ffi call
    use crate::query_dsl::RunQueryDsl;
    #[cfg(not(miri))] // ffi call
    use crate::test_helpers::connection;
    #[cfg(not(miri))] // ffi call
    use crate::{IntoSql, dsl::sql};
    use serde_json::{Value, json};
    use sql_types::{Json, Jsonb};

    // Helper function to create the correct JsonbHeader based on the payload size
    pub(super) fn create_jsonb_header(
        element_type: u8,
        payload_size: usize,
    ) -> Result<Vec<u8>, Box<dyn core::error::Error + Send + Sync>> {
        let mut buffer = Vec::new();
        jsonb::write_jsonb_header(&mut buffer, element_type, payload_size)?;
        Ok(buffer)
    }

    #[diesel_test_helper::test]
    #[cfg(not(miri))] // ffi call
    fn regression_float_without_a_fraction_is_written_invalid() {
        let conn = &mut connection();
        for value in [
            json!(3.0),
            json!(-0.0),
            json!(1.5e300),
            // an exponent and no fraction digit, so the text carries no `.`
            json!(1e-7),
            json!(1e300),
        ] {
            let blob = diesel::select(sql::<sql_types::Binary>("").bind::<Jsonb, _>(value.clone()))
                .get_result::<Vec<u8>>(conn)
                .unwrap();
            let valid = diesel::select(
                sql::<sql_types::Integer>("json_valid(")
                    .bind::<sql_types::Binary, _>(blob.clone())
                    .sql(", 8)"),
            )
            .get_result::<i32>(conn)
            .unwrap();
            assert_eq!(
                valid, 1,
                "sqlite rejects the blob written for {value}: {blob:02X?}"
            );
            let back = diesel::select(sql::<Jsonb>("").bind::<sql_types::Binary, _>(blob.clone()))
                .get_result::<Value>(conn)
                .unwrap_or_else(|error| panic!("{value} does not read back: {error}"));
            assert_eq!(back, value, "{blob:02X?}");
        }
    }
    #[diesel_test_helper::test]
    #[cfg(not(miri))] // ffi call
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
    fn test_read_jsonb_textraw() {
        // JSONB_TEXTRAW with payload "foo"
        let mut data = Vec::new();
        data.extend(create_jsonb_header(JSONB_TEXTRAW, 0x03).unwrap());
        data.extend_from_slice(b"foo");

        let result = read_jsonb_value(&data).unwrap().0;
        assert_eq!(result, json!("foo"));
    }

    #[diesel_test_helper::test]
    fn test_read_jsonb_object_with_textraw_key() {
        // JSONB_OBJECT with a TEXTRAW key and value
        let mut data = Vec::new();
        data.extend(create_jsonb_header(JSONB_OBJECT, 0x06).unwrap());
        data.extend(create_jsonb_header(JSONB_TEXTRAW, 0x01).unwrap());
        data.extend_from_slice(b"a");
        data.extend(create_jsonb_header(JSONB_TEXTRAW, 0x03).unwrap());
        data.extend_from_slice(b"bar");

        let result = read_jsonb_value(&data).unwrap().0;
        assert_eq!(result, json!({"a": "bar"}));
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
        write_jsonb_value(&json!(input_string), &mut buffer).unwrap();

        let mut expected_buffer = Vec::new();
        expected_buffer.extend(create_jsonb_header(JSONB_TEXT, 0x05).unwrap());
        expected_buffer.extend_from_slice(b"hello");

        assert_eq!(buffer, expected_buffer);
    }

    #[diesel_test_helper::test]
    fn test_write_jsonb_textj() {
        let mut buffer = Vec::new();
        let input_string = "hello\nworld"; // Contains a newline, requires escaping
        write_jsonb_value(&json!(input_string), &mut buffer).unwrap();

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
    #[cfg(not(miri))] // ffi call
    fn jsonb_to_sql_bool() {
        let conn = &mut connection();
        let res = diesel::select(json!(true).into_sql::<Jsonb>().eq(&sql("jsonb('true')")))
            .get_result::<bool>(conn)
            .unwrap();
        assert!(res);
    }

    #[diesel_test_helper::test]
    #[cfg(not(miri))] // ffi call
    fn jsonb_to_sql_null() {
        let conn = &mut connection();
        let res = diesel::select(json!(null).into_sql::<Jsonb>().eq(&sql("jsonb('null')")))
            .get_result::<bool>(conn)
            .unwrap();
        assert!(res);
    }

    #[diesel_test_helper::test]
    #[cfg(not(miri))] // ffi call
    fn jsonb_to_sql_integer() {
        let conn = &mut connection();
        let res = diesel::select(json!(42).into_sql::<Jsonb>().eq(&sql("jsonb('42')")))
            .get_result::<bool>(conn)
            .unwrap();
        assert!(res);
    }

    #[diesel_test_helper::test]
    #[cfg(not(miri))] // ffi call
    fn jsonb_to_sql_float() {
        let conn = &mut connection();
        let res = diesel::select(json!(42.23).into_sql::<Jsonb>().eq(&sql("jsonb('42.23')")))
            .get_result::<bool>(conn)
            .unwrap();
        assert!(res);
    }

    #[diesel_test_helper::test]
    #[cfg(not(miri))] // ffi call
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
    #[cfg(not(miri))] // ffi call
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
    #[cfg(not(miri))] // ffi call
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
    #[cfg(not(miri))] // ffi call
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
    #[cfg(not(miri))] // ffi call
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
    #[cfg(not(miri))] // ffi call
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
    #[cfg(not(miri))] // ffi call
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
    #[cfg(not(miri))] // ffi call
    fn jsonb_from_sql_null() {
        let conn = &mut connection();
        let res = diesel::select(sql::<Jsonb>("jsonb('null')"))
            .get_result::<serde_json::Value>(conn)
            .unwrap();
        assert_eq!(res, serde_json::json!(null));
    }

    #[diesel_test_helper::test]
    #[cfg(not(miri))] // ffi call
    fn jsonb_from_sql_true() {
        let conn = &mut connection();
        let res = diesel::select(sql::<Jsonb>("jsonb('true')"))
            .get_result::<serde_json::Value>(conn)
            .unwrap();
        assert_eq!(res, serde_json::json!(true));
    }

    #[diesel_test_helper::test]
    #[cfg(not(miri))] // ffi call
    fn jsonb_from_sql_false() {
        let conn = &mut connection();
        let res = diesel::select(sql::<Jsonb>("jsonb('false')"))
            .get_result::<serde_json::Value>(conn)
            .unwrap();
        assert_eq!(res, serde_json::json!(false));
    }

    #[diesel_test_helper::test]
    #[cfg(not(miri))] // ffi call
    fn jsonb_from_sql_int() {
        let conn = &mut connection();
        let res = diesel::select(sql::<Jsonb>("jsonb('42')"))
            .get_result::<serde_json::Value>(conn)
            .unwrap();
        assert_eq!(res, serde_json::json!(42));
    }

    #[diesel_test_helper::test]
    #[cfg(not(miri))] // ffi call
    fn jsonb_from_sql_float() {
        let conn = &mut connection();
        let res = diesel::select(sql::<Jsonb>("jsonb('42.23')"))
            .get_result::<serde_json::Value>(conn)
            .unwrap();
        assert_eq!(res, serde_json::json!(42.23));
    }

    #[diesel_test_helper::test]
    #[cfg(not(miri))] // ffi call
    fn jsonb_from_sql_object() {
        let conn = &mut connection();
        let res = diesel::select(sql::<Jsonb>("jsonb('{\"key\": \"value\"}')"))
            .get_result::<serde_json::Value>(conn)
            .unwrap();
        assert_eq!(res, serde_json::json!({"key": "value"}));
    }

    #[diesel_test_helper::test]
    #[cfg(not(miri))] // ffi call
    fn jsonb_from_sql_array() {
        let conn = &mut connection();
        let res = diesel::select(sql::<Jsonb>("jsonb('[1, 2, 3]')"))
            .get_result::<serde_json::Value>(conn)
            .unwrap();
        assert_eq!(res, serde_json::json!([1, 2, 3]));
    }

    #[diesel_test_helper::test]
    #[cfg(not(miri))] // ffi call
    fn jsonb_from_sql_nested_objects() {
        let conn = &mut connection();
        let res = diesel::select(sql::<Jsonb>("jsonb('{\"outer\": {\"inner\": 42}}')"))
            .get_result::<serde_json::Value>(conn)
            .unwrap();
        assert_eq!(res, serde_json::json!({"outer": {"inner": 42}}));
    }

    #[diesel_test_helper::test]
    #[cfg(not(miri))] // ffi call
    fn jsonb_from_sql_nested_arrays() {
        let conn = &mut connection();
        let res = diesel::select(sql::<Jsonb>("jsonb('[[1, 2], [3, 4]]')"))
            .get_result::<serde_json::Value>(conn)
            .unwrap();
        assert_eq!(res, serde_json::json!([[1, 2], [3, 4]]));
    }

    #[diesel_test_helper::test]
    #[cfg(not(miri))] // ffi call
    fn jsonb_from_sql_nested_arrays_in_objects() {
        let conn = &mut connection();
        let res = diesel::select(sql::<Jsonb>("jsonb('{\"array\": [1, 2, 3]}')"))
            .get_result::<serde_json::Value>(conn)
            .unwrap();
        assert_eq!(res, serde_json::json!({"array": [1, 2, 3]}));
    }

    #[diesel_test_helper::test]
    #[cfg(not(miri))] // ffi call
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
    #[cfg(not(miri))] // ffi call
    fn jsonb_from_sql_text() {
        let conn = &mut connection();
        let res = diesel::select(sql::<Jsonb>("jsonb('\"hello\"')"))
            .get_result::<serde_json::Value>(conn)
            .unwrap();
        assert_eq!(res, serde_json::json!("hello"));
    }

    #[diesel_test_helper::test]
    #[cfg(not(miri))] // ffi call
    fn jsonb_from_sql_textj() {
        let conn = &mut connection();
        let res = diesel::select(sql::<Jsonb>("jsonb('\"hello\\nworld\"')"))
            .get_result::<serde_json::Value>(conn)
            .unwrap();
        assert_eq!(res, serde_json::json!("hello\nworld"));
    }

    #[diesel_test_helper::test]
    #[cfg(not(miri))] // ffi call
    fn bad_json_from_sql() {
        let conn = &mut connection();
        let res = diesel::select(json!(true).into_sql::<Json>().eq(&sql("json('boom')")))
            .get_result::<bool>(conn);
        assert_eq!(res.unwrap_err().to_string(), "malformed JSON");
    }

    #[diesel_test_helper::test]
    #[cfg(not(miri))] // ffi call
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

    #[cfg(all(
        not(miri),
        not(all(target_family = "wasm", target_os = "unknown")),
        unix
    ))]
    const RECURSION_DEPTH: usize = 2000;

    #[cfg(all(
        not(miri),
        any(windows, all(target_family = "wasm", target_os = "unknown"))
    ))]
    const RECURSION_DEPTH: usize = 1000;

    #[cfg(any(
        miri,
        all(
            not(unix),
            not(windows),
            not(all(target_family = "wasm", target_os = "unknown"))
        )
    ))]
    const RECURSION_DEPTH: usize = 10;

    #[diesel_test_helper::test]
    fn guard_against_stackoverflow_array() {
        let mut value = serde_json::Value::Number(42.into());
        for i in 0..RECURSION_DEPTH {
            value = serde_json::Value::Array(vec![value, serde_json::Value::Number(i.into())]);
        }
        // We compare the encoded buffer for both values here
        // as serde_json otherwise runs into stackoverflows itself
        let mut expected_buffer = Vec::new();
        write_jsonb_value(&value, &mut expected_buffer).unwrap();
        let res = read_jsonb_value(&expected_buffer).unwrap().0;

        let mut buffer = Vec::new();
        write_jsonb_value(&res, &mut buffer).unwrap();
        assert_eq!(expected_buffer, buffer);
    }

    #[diesel_test_helper::test]
    fn guard_against_stackoverflow_object() {
        let mut value = serde_json::Value::Number(42.into());
        for i in 0..RECURSION_DEPTH {
            let mut map = serde_json::Map::new();
            map.insert(format!("key_{i}"), value);
            value = serde_json::Value::Object(map);
        }

        // We compare the encoded buffer for both values here
        // as serde_json otherwise runs into stackoverflows itself
        let mut expected_buffer = Vec::new();
        write_jsonb_value(&value, &mut expected_buffer).unwrap();

        let res = read_jsonb_value(&expected_buffer).unwrap().0;

        let mut buffer = Vec::new();
        write_jsonb_value(&res, &mut buffer).unwrap();
        assert_eq!(expected_buffer, buffer);
    }

    #[diesel_test_helper::test]
    fn guard_against_stackoverflow_mixed() {
        let mut value = serde_json::Value::Number(42.into());
        for i in 0_usize..2000 {
            if i.is_multiple_of(2) {
                let mut map = serde_json::Map::new();
                map.insert(format!("key_{i}"), value);
                value = serde_json::Value::Object(map);
            } else {
                value = serde_json::Value::Array(vec![value]);
            }
        }
        // We compare the encoded buffer for both values here
        // as serde_json otherwise runs into stackoverflows itself
        let mut expected_buffer = Vec::new();
        write_jsonb_value(&value, &mut expected_buffer).unwrap();
        let res = read_jsonb_value(&expected_buffer).unwrap().0;
        let mut buffer = Vec::new();
        write_jsonb_value(&res, &mut buffer).unwrap();
        assert_eq!(expected_buffer, buffer);
    }

    #[diesel_test_helper::test]
    #[cfg(not(miri))] // ffi call
    fn dangling_bytes_result_in_error() {
        let mut value = Vec::<u8>::new();
        value.extend(create_jsonb_header(JSONB_INT, 1).unwrap());
        value.push(b'1');
        value.push(42);
        assert_eq!(value.len(), 3);
        let conn = &mut connection();
        let res = diesel::select(
            crate::dsl::sql::<sql_types::Jsonb>("jsonb(?)").bind::<sql_types::Binary, _>(value),
        )
        .get_result::<serde_json::Value>(conn);
        assert!(res.is_err(), "{:?}", res.unwrap());
    }

    #[diesel_test_helper::test]
    fn object_key_without_value_results_in_error() {
        let mut value = Vec::new();
        value.extend(create_jsonb_header(JSONB_OBJECT, 2).unwrap());
        value.extend(create_jsonb_header(JSONB_TEXT, 1).unwrap());
        value.push(b'a');
        let res = read_jsonb_value(&value);
        assert!(res.is_err(), "{:?}", res.unwrap());
    }

    #[diesel_test_helper::test]
    fn check_invalid_size_header() {
        // 9-byte JSONB blob: first byte 0xFB (size_hint nibble = 0x0F),
        // bytes 1..9 = 0xFF -> encoded payload length = u64::MAX.
        let res = read_jsonb_value(&[0xFB, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF]);
        assert!(res.is_err());
    }

    #[diesel_test_helper::test]
    fn nested_container_cannot_cross_parent_boundary() {
        let res = read_jsonb_value(&[0x3B, 0x1B, 0x1B, JSONB_NULL]);
        assert!(res.is_err(), "{:?}", res.unwrap());
    }

    #[diesel_test_helper::test]
    fn check_signed_integer() {
        let mut buf = Vec::new();
        write_jsonb_value(&json!(-42), &mut buf).unwrap();
        let mut expected = create_jsonb_header(JSONB_INT, 3).unwrap();
        expected.extend(b"-42");
        assert_eq!(buf, expected);
    }
}
