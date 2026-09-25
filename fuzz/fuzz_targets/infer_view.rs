#![no_main]
//! A view column diesel infers NOT NULL must never be NULL in SQLite.

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if let Err(violation) = diesel_fuzz::infer_view::run_case(data) {
        panic!("{violation}");
    }
});
