extern crate trybuild;

#[test]
fn trybuild() {
    let t = trybuild::TestCases::new();
    t.compile_fail("tests/fail/*.rs");
    t.compile_fail("tests/fail/derive/*.rs");
    t.compile_fail("tests/fail/derive_deprecated/*.rs");
}
