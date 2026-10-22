#![no_main]
//! SQLite blob reads, seeks, and closing must agree with a byte-slice model.

use diesel_fuzz::sqlite_blob::BlobInput;
use libfuzzer_sys::fuzz_target;

fuzz_target!(|input: BlobInput<'_>| {
    if let Err(violation) = diesel_fuzz::sqlite_blob::run_case(&input) {
        panic!("{violation}");
    }
});
