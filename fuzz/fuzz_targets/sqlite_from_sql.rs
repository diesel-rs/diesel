#![no_main]
//! Decoding a bound sqlite value as any type must never panic.

use arbitrary::Arbitrary;
use libfuzzer_sys::fuzz_target;

#[derive(Arbitrary, Debug)]
struct Input<'a> {
    selector: u8,
    kind: u8,
    bytes: &'a [u8],
}

fuzz_target!(|input: Input<'_>| {
    diesel_fuzz::sqlite::decode_case(input.selector, input.kind, input.bytes);
});
