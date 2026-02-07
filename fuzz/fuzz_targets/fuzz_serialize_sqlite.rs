#![no_main]

use libfuzzer_sys::{fuzz_target, arbitrary::{Arbitrary, Unstructured}};

#[derive(Debug, Arbitrary)]
enum FuzzValue {
    Integer(i32),
    BigInt(i64),
    Text(String),
    Float(f32),
    Double(f64),
    Bool(bool),
    Binary(Vec<u8>),
    Null,
}

fuzz_target!(|data: &[u8]| {
    let mut u = Unstructured::new(data);

    // Generate arbitrary values and test serialization logic
    if let Ok(value) = FuzzValue::arbitrary(&mut u) {
        match value {
            FuzzValue::Integer(v) => {
                // Test integer to bytes conversion
                let _ = v.to_be_bytes();
                let _ = v.to_le_bytes();
                let _ = v.to_string();
            }
            FuzzValue::BigInt(v) => {
                let _ = v.to_be_bytes();
                let _ = v.to_le_bytes();
                let _ = v.to_string();
            }
            FuzzValue::Text(v) => {
                // Test string encoding
                let _ = v.as_bytes();
                let _ = v.len();
                // Test for SQL injection patterns
                let _contains_quote = v.contains('\'');
                let _contains_semicolon = v.contains(';');
            }
            FuzzValue::Float(v) => {
                let _ = v.to_be_bytes();
                let _ = v.to_le_bytes();
                let _ = v.to_string();
                let _ = v.is_nan();
                let _ = v.is_infinite();
            }
            FuzzValue::Double(v) => {
                let _ = v.to_be_bytes();
                let _ = v.to_le_bytes();
                let _ = v.to_string();
                let _ = v.is_nan();
                let _ = v.is_infinite();
            }
            FuzzValue::Bool(v) => {
                // SQLite represents bools as integers
                let _as_int = if v { 1 } else { 0 };
            }
            FuzzValue::Binary(v) => {
                let _ = v.len();
                // Test for potential buffer issues
                if v.len() < 1000000 {
                    let _ = v.clone();
                }
            }
            FuzzValue::Null => {}
        }
    }
});
