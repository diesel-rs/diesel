#![no_main]
//! Decoding an arbitrary blob as jsonb must never panic.

use libfuzzer_sys::fuzz_target;

fuzz_target!(|blob: &[u8]| {
    diesel_fuzz::sqlite::with_conn(|conn| {
        let _ = diesel_fuzz::sqlite::decode_jsonb(conn, blob);
    });
});
