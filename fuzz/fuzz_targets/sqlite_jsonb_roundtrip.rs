#![no_main]
//! What diesel writes, diesel must read back unchanged and sqlite must accept.

use diesel_fuzz::document::Document;
use libfuzzer_sys::fuzz_target;

fuzz_target!(|document: Document| {
    let value = serde_json::Value::from(&document);
    diesel_fuzz::sqlite::with_conn(|conn| {
        if let Err(violation) = diesel_fuzz::sqlite::roundtrip_jsonb(conn, &value) {
            panic!("{violation}");
        }
        if let Err(violation) = diesel_fuzz::sqlite::roundtrip_json(conn, &value) {
            panic!("{violation}");
        }
    });
});
