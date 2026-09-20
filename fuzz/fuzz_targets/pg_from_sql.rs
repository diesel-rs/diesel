#![no_main]
//! Decoding arbitrary bytes as any postgres type must never panic.

use arbitrary::Arbitrary;
use libfuzzer_sys::fuzz_target;
use std::num::NonZeroU32;

#[derive(Arbitrary, Debug)]
struct Input<'a> {
    selector: u8,
    oid: u32,
    bytes: &'a [u8],
}

fuzz_target!(|input: Input<'_>| {
    let oid = NonZeroU32::new(input.oid).unwrap_or(NonZeroU32::MIN);
    diesel_fuzz::pg::decode_case(input.selector, oid, input.bytes);
});
