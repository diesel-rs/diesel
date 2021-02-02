use crate::schema::connection_without_transaction;
use diesel::{sql_query, RunQueryDsl};

#[test]
fn test_load_extension() {
    let conn = connection_without_transaction();

    // disable extension loading
    conn.disable_load_extension().unwrap();

    // try loading module without enabling extension loading
    let result = sql_query("SELECT load_extension('/tmp/foo');")
        .execute(&conn)
        .expect_err("should have been error");

    assert_eq!(&format!("{}", &result), "not authorized");

    // enable extension loading
    conn.enable_load_extension().unwrap();

    // try loading module without enabling extension loading
    let result = sql_query("SELECT load_extension('/tmp/foo');")
        .execute(&conn)
        .expect_err("should have been error");

    assert_ne!(&format!("{}", &result), "not authorized");
}
