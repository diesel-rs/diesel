#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    // Test JSON deserialization for SQLite
    if let Ok(json_str) = std::str::from_utf8(data) {
        // Try to parse as JSON
        if let Ok(value) = serde_json::from_str::<serde_json::Value>(json_str) {
            // Test nested JSON access patterns
            match &value {
                serde_json::Value::Object(obj) => {
                    for (key, val) in obj.iter().take(10) {
                        // Test key access patterns (potential for panics in complex structures)
                        let _key_len = key.len();

                        // Test nested object access
                        if val.is_object() {
                            let _ = val.get(key);
                        }
                    }
                }
                serde_json::Value::Array(arr) => {
                    // Test array access patterns
                    for item in arr.iter().take(100) {
                        // Limit iteration to prevent excessive processing
                        let _ = item.is_null();
                        let _ = item.is_boolean();
                        let _ = item.is_number();
                        let _ = item.is_string();
                    }
                }
                _ => {}
            }

            // Test JSON serialization roundtrip
            if let Ok(serialized) = serde_json::to_string(&value) {
                let _ = serde_json::from_str::<serde_json::Value>(&serialized);
            }
        }
    }

    // Also test invalid JSON to ensure proper error handling
    let _ = serde_json::from_slice::<serde_json::Value>(data);
});
