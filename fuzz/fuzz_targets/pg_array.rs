#![no_main]
//! PostgreSQL arrays diesel writes and reads must match a model of the binary format.

use diesel_fuzz::pg_array::ArrayInput;
use libfuzzer_sys::fuzz_target;

fuzz_target!(|input: ArrayInput| {
    if let Err(violation) = diesel_fuzz::pg_array::run_case(&input) {
        panic!("{violation}");
    }
});
