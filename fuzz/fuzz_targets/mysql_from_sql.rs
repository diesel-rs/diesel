#![no_main]
//! Decoding arbitrary bytes as any mysql type must never panic.

use arbitrary::Arbitrary;
use libfuzzer_sys::fuzz_target;

#[derive(Arbitrary, Debug)]
struct Input<'a> {
    selector: u8,
    type_selector: u8,
    bytes: &'a [u8],
}

fuzz_target!(|input: Input<'_>| {
    diesel_fuzz::mysql::decode_case(input.selector, input.type_selector, input.bytes);
});
