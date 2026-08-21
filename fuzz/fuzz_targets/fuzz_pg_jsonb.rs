#![no_main]

use libfuzzer_sys::fuzz_target;

use core::num::NonZeroU32;
use diesel::deserialize::FromSql;
use diesel::pg::{Pg, PgValue};
use diesel::sql_types;

// Fuzz diesel's PostgreSQL JSON/JSONB wire protocol parser.
//
// For JSON: serde_json::from_slice(value.as_bytes())
// For JSONB: checks bytes[0] == 1 (version byte), then serde_json::from_slice(&bytes[1..])
//
// Key risk: JSONB parsing accesses bytes[0] without bounds check.
// Empty input will panic with index-out-of-bounds.
// See: diesel/src/pg/types/json.rs line 32
//
// Targets: diesel/src/pg/types/json.rs (FromSql<Json/Jsonb, Pg>)

fuzz_target!(|data: &[u8]| {
    // Test JSON (plain text JSON)
    let oid = NonZeroU32::new(114).unwrap(); // JSONOID
    let value = PgValue::new(data, &oid);
    let _ = <serde_json::Value as FromSql<sql_types::Json, Pg>>::from_sql(value);

    // Test JSONB (binary format with version byte prefix)
    let oid = NonZeroU32::new(3802).unwrap(); // JSONBOID
    let value = PgValue::new(data, &oid);
    let _ = <serde_json::Value as FromSql<sql_types::Jsonb, Pg>>::from_sql(value);
});
