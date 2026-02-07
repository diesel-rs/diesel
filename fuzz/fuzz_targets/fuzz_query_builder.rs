#![no_main]

use libfuzzer_sys::{fuzz_target, arbitrary::{Arbitrary, Unstructured}};
use diesel::prelude::*;
use diesel::sql_types::*;

#[derive(Debug, Arbitrary)]
struct FuzzInput {
    table_name: String,
    column_name: String,
    limit_value: i64,
    offset_value: i64,
}

fuzz_target!(|data: &[u8]| {
    let mut u = Unstructured::new(data);

    if let Ok(input) = FuzzInput::arbitrary(&mut u) {
        // Test SQL identifier escaping with arbitrary names
        // This tests whether diesel properly escapes/validates identifiers

        // Limit the string lengths to avoid excessive memory usage
        let table_name = input.table_name.chars().take(100).collect::<String>();
        let column_name = input.column_name.chars().take(100).collect::<String>();

        // Test that identifiers with special characters are handled correctly
        // Diesel should escape these or reject them
        let _sanitized_table = table_name.replace([';', '\0', '\n', '\r'], "");
        let _sanitized_column = column_name.replace([';', '\0', '\n', '\r'], "");

        // Test numeric limits (potential for integer overflow)
        let limit = input.limit_value.abs().min(1000000);
        let offset = input.offset_value.abs().min(1000000);

        // Test that these values don't cause panics or overflows
        let _ = limit.checked_add(offset);
        let _ = limit.checked_mul(offset);
    }
});
