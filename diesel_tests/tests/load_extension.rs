use crate::schema::connection;

#[test]
fn check_load_extension() {
    let conn = connection();

    let r = dbg!(conn.load_extension("/tmp/test.so", None));
    assert!(r.is_err());
    // As we don't have any actual extension here, we just check if the actual loading fails
    assert!(r
        .unwrap_err()
        .to_string()
        .ends_with("No such file or directory"));
    let r = dbg!(conn.load_extension("/tmp/test.so", Some("foo")));
    assert!(r.is_err());
    // As we don't have any actual extension here, we just check if the actual loading fails
    assert!(r
        .unwrap_err()
        .to_string()
        .ends_with("No such file or directory"));
}
