#![no_main]

use libfuzzer_sys::fuzz_target;

use core::num::NonZeroU32;
use core::ops::Bound;
use diesel::deserialize::FromSql;
use diesel::pg::{Pg, PgValue};
use diesel::sql_types::Integer;

// Fuzz diesel's PostgreSQL range wire protocol parser.
//
// (Bound<T>, Bound<T>)::from_sql parses the PG binary range format:
//   [flags: u8] (bitfield: EMPTY, LB_INC, UB_INC, LB_INF, UB_INF, ...)
//   If lower bound present: [elem_size: i32] [elem_data: bytes]
//   If upper bound present: [elem_size: i32] [elem_data: bytes]
//
// Key risk: split_at(elem_size.try_into()?) on lower bound can panic
// if elem_size exceeds remaining buffer.
//
// Targets: diesel/src/pg/types/ranges.rs (FromSql<Range<ST>, Pg>)

fuzz_target!(|data: &[u8]| {
    let oid = NonZeroU32::new(3904).unwrap(); // INT4RANGEOID
    let value = PgValue::new(data, &oid);
    let _ = <(Bound<i32>, Bound<i32>) as FromSql<diesel::pg::sql_types::Range<Integer>, Pg>>::from_sql(value);
});
