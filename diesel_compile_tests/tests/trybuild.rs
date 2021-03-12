extern crate trybuild;

#[test]
fn trybuild() {
    let t = trybuild::TestCases::new();
    t.compile_fail("tests/fail/ensures_derive_queryable_by_name_valid_type.rs");
}
