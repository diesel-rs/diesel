use crate::schema::connection_without_transaction;
use diesel::{sql_query, RunQueryDsl};

fn is_extension_loading_supported() -> bool {
    std::env::var("DIESEL_TEST_SQLITE_EXTENSIONS_DISABLED").is_err()
}

#[test]
fn test_load_extension() {
    let mut conn = connection_without_transaction();

    // disable extension loading
    conn.disable_load_extension().unwrap();

    // try loading module without enabling extension loading
    let result = sql_query("SELECT load_extension('/tmp/foo');")
        .execute(&mut conn)
        .expect_err("should have been error");

    if is_extension_loading_supported() {
        assert_eq!(&format!("{}", &result), "not authorized");
    } else {
        assert_eq!(&format!("{}", &result), "no such function: load_extension");
    }

    // enable extension loading
    conn.enable_load_extension().unwrap();

    // try loading module without enabling extension loading
    let result = sql_query("SELECT load_extension('/tmp/foo');")
        .execute(&mut conn)
        .expect_err("should have been error");

    if is_extension_loading_supported() {
        assert_ne!(&format!("{}", &result), "not authorized");
    } else {
        assert_eq!(&format!("{}", &result), "no such function: load_extension");
    }
}

#[test]
fn test_load_extensions_transaction() {
    let mut conn = connection_without_transaction();

    // We expect this to fail because extension likely doesn't exist
    // But verify that it disabled the extension loading
    let _ = conn.load_extensions(&["/non/existent/extension"]);

    // Check it is disabled
    let result = sql_query("SELECT load_extension('/non/existent/extension');")
        .execute(&mut conn)
        .expect_err("should have been error");

    if is_extension_loading_supported() {
        assert_eq!(&format!("{}", &result), "not authorized");
    } else {
        assert_eq!(&format!("{}", &result), "no such function: load_extension");
    }
}

#[test]
fn test_load_uuid_extension() {
    let mut conn = connection_without_transaction();

    // We attempt to load the uuid extension
    // This might fail if the extension is not installed on the system
    match conn.load_extensions(&["uuid"]) {
        Ok(_) => {
            // If loading succeeded, the extension functions should be available
            // and the load_extension function should be disabled

            // Check function availability
            let result = sql_query("SELECT uuid();").execute(&mut conn);
            assert!(result.is_ok(), "uuid() function should be available");

            // Check that load_extension is disabled
            let result = sql_query("SELECT load_extension('uuid');")
                .execute(&mut conn)
                .expect_err("should have been error");
            assert_eq!(&format!("{}", &result), "not authorized");
        }
        Err(e) => {
            // If loading failed, it should NOT be because of authorization
            assert_ne!(
                format!("{}", e),
                "not authorized",
                "Should have been authorized to load extension"
            );
        }
    }
}
