use crate::schema::connection_without_transaction;
use diesel::sql_types::Text;
use diesel::sqlite::SqliteExtension;

struct SqliteUUIDExtension;
impl SqliteExtension for SqliteUUIDExtension {
    const FILENAME: &'static std::ffi::CStr = c"uuid";
}

struct SqliteMathFunctionsExtension;
impl SqliteExtension for SqliteMathFunctionsExtension {
    const FILENAME: &'static std::ffi::CStr = c"extension-functions";
}

#[test]
fn test_load_all_extensions() {
    let env_var = std::env::var("DIESEL_TEST_SQLITE_EXTENSIONS_DISABLED");
    if env_var.is_err() {
        eprintln!("Skipping test_load_all_extensions");
        return;
    }
    let val = env_var.unwrap();
    let restricted_build = match val.as_str() {
        "1" => true,
        "0" => false,
        _ => panic!("DIESEL_TEST_SQLITE_EXTENSIONS_DISABLED must be 0 or 1"),
    };

    let mut conn = connection_without_transaction();

    test_extension::<SqliteUUIDExtension, _>(&mut conn, restricted_build, "uuid", |conn| {
        use diesel::RunQueryDsl;
        let uuid_result =
            diesel::select(diesel::dsl::sql::<Text>("uuid()")).get_result::<String>(conn);
        assert!(uuid_result.is_ok(), "uuid() function should be available");
        let uuid_value = uuid_result.unwrap();
        assert_eq!(uuid_value.len(), 36);
    });

    test_extension::<SqliteMathFunctionsExtension, _>(
        &mut conn,
        restricted_build,
        "extension-functions",
        |conn| {
            use diesel::RunQueryDsl;
            let cos_result =
                diesel::select(diesel::dsl::sql::<diesel::sql_types::Double>("cos(0)"))
                    .get_result::<f64>(conn);
            // If compiled with math functions, this works.
            if let Ok(val) = cos_result {
                assert!((val - 1.0).abs() < 1e-6);
            }
        },
    );
}

fn test_extension<E, F>(
    conn: &mut diesel::sqlite::SqliteConnection,
    restricted_build: bool,
    name: &str,
    validation: F,
) where
    E: SqliteExtension,
    F: FnOnce(&mut diesel::sqlite::SqliteConnection),
{
    match conn.load_extension::<E>() {
        Ok(_) => {
            if restricted_build {
                panic!(
                    "Extension loading should fail in restricted build: {}",
                    name
                );
            }
            validation(conn);
        }
        Err(e) => {
            let error_string = format!("{}", e);
            if restricted_build {
                assert_eq!(
                    error_string, "no such function: load_extension",
                    "Should fail with specific error in restricted build for {}",
                    name
                );
            } else {
                panic!(
                    "Extension loading failed unexpectedly for {}: {}",
                    name, error_string
                );
            }
        }
    }
}
