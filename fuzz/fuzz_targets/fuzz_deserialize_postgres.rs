#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    // Fuzz UTF-8 decoding with different encodings
    let _ = std::str::from_utf8(data);

    // Fuzz numeric parsing
    if let Ok(s) = std::str::from_utf8(data) {
        let _ = s.parse::<i32>();
        let _ = s.parse::<i64>();
        let _ = s.parse::<f32>();
        let _ = s.parse::<f64>();
        let _ = s.parse::<bool>();
    }

    // Fuzz integer deserialization with different byte orders
    if data.len() >= 2 {
        let _ = i16::from_be_bytes([data[0], data[1]]);
        let _ = i16::from_le_bytes([data[0], data[1]]);
    }

    if data.len() >= 4 {
        let _ = i32::from_be_bytes([data[0], data[1], data[2], data[3]]);
        let _ = f32::from_be_bytes([data[0], data[1], data[2], data[3]]);
    }
});
