use diesel::{sql_query, Connection, RunQueryDsl, SqliteConnection};

fn conn() -> SqliteConnection {
    SqliteConnection::establish(":memory:").unwrap()
}

#[test]
fn test_load_extension() {
    let conn = conn();

    // disable extension loading
    conn.enable_load_extension(false).unwrap();

    // try loading module without enabling extension loading
    let result = sql_query("SELECT load_extension('/tmp/foo');")
        .execute(&conn)
        .expect_err("should have been error");

    assert_eq!(&format!("{}", &result), "not authorized");

    // enable extension loading
    conn.enable_load_extension(true).unwrap();

    // try loading module without enabling extension loading
    let result = sql_query("SELECT load_extension('/tmp/foo');")
        .execute(&conn)
        .expect_err("should have been error");

    assert_ne!(&format!("{}", &result), "not authorized");
}
