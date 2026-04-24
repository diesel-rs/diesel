#![no_main]

use libfuzzer_sys::fuzz_target;

use core::num::NonZeroU32;
use diesel::deserialize::FromSql;
use diesel::pg::{Pg, PgValue};
use diesel::sql_types::{Array, Integer, Text};

// Fuzz diesel's PostgreSQL array wire protocol parser.
//
// Vec<T>::from_sql parses the PG binary array format:
//   [num_dimensions: i32] [has_null: i32] [oid: i32]
//   [num_elements: i32] [lower_bound: i32]
//   For each element: [elem_size: i32] [elem_data: bytes]
//
// Key risk: split_at(elem_size.try_into()?) can panic if elem_size
// exceeds the remaining buffer length.
//
// Targets: diesel/src/pg/types/array.rs (FromSql<Array<ST>, Pg> for Vec<T>)

fuzz_target!(|data: &[u8]| {
    let oid = NonZeroU32::new(1007).unwrap(); // INT4ARRAYOID
    let value = PgValue::new(data, &oid);
    let _ = <Vec<i32> as FromSql<Array<Integer>, Pg>>::from_sql(value);

    // Also test with text arrays (different element parsing)
    let oid = NonZeroU32::new(1009).unwrap(); // TEXTARRAYOID
    let value = PgValue::new(data, &oid);
    let _ = <Vec<String> as FromSql<Array<Text>, Pg>>::from_sql(value);
});
