#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    // Fuzz UTF-8 string decoding (common in text columns)
    let _ = std::str::from_utf8(data);

    // Fuzz integer parsing from bytes
    if data.len() >= 4 {
        let _ = i32::from_be_bytes([data[0], data[1], data[2], data[3]]);
        let _ = i32::from_le_bytes([data[0], data[1], data[2], data[3]]);
    }

    if data.len() >= 8 {
        let _ = i64::from_be_bytes([
            data[0], data[1], data[2], data[3],
            data[4], data[5], data[6], data[7],
        ]);
        let _ = f64::from_be_bytes([
            data[0], data[1], data[2], data[3],
            data[4], data[5], data[6], data[7],
        ]);
    }

    // Fuzz boolean interpretation (SQLite uses integers for bools)
    if !data.is_empty() {
        let _ = data[0] != 0;
    }
});
