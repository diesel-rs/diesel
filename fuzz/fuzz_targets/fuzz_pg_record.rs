#![no_main]

use libfuzzer_sys::fuzz_target;

use core::num::NonZeroU32;
use diesel::deserialize::FromSql;
use diesel::pg::{Pg, PgValue};
use diesel::sql_types::{Integer, Record, Text};

// Fuzz diesel's PostgreSQL composite type (record/tuple) wire protocol parser.
//
// Tuple::from_sql parses the PG binary record format:
//   [num_elements: i32]
//   For each element: [oid: u32] [num_bytes: i32] [data: bytes]
//
// Key risks:
//   - NonZeroU32::new(oid).expect("Oid's aren't zero") panics on OID=0
//   - split_at(num_bytes.try_into()?) can panic on oversized num_bytes
//
// Targets: diesel/src/pg/types/record.rs (FromSql<Record<(..)>, Pg>)

fuzz_target!(|data: &[u8]| {
    let oid = NonZeroU32::new(2249).unwrap(); // RECORDOID
    let value = PgValue::new(data, &oid);

    // Test 2-element tuple
    let _ = <(i32, i32) as FromSql<Record<(Integer, Integer)>, Pg>>::from_sql(value);

    // Test 3-element tuple with mixed types
    let value = PgValue::new(data, &oid);
    let _ = <(i32, String, i32) as FromSql<Record<(Integer, Text, Integer)>, Pg>>::from_sql(value);
});
