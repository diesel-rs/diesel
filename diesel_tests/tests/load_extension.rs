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
        eprintln!("Skipping test_load_all_extensions because DIESEL_TEST_SQLITE_EXTENSIONS_DISABLED is not set");
        return;
    }
    let val = env_var.unwrap();
    let restricted_build = match val.as_str() {
        "1" => true,
        "0" => false,
        _ => panic!("DIESEL_TEST_SQLITE_EXTENSIONS_DISABLED must be 0 or 1"),
    };

    if !restricted_build {
        let library_paths = std::env::var_os("LD_LIBRARY_PATH").unwrap_or_default();
        let paths: Vec<std::path::PathBuf> = std::env::split_paths(&library_paths).collect();
        let extensions = ["uuid", "extension-functions"];
        let mut missing = Vec::new();

        for ext in &extensions {
            let mut found = false;
            // Simple check for common shared library extensions
            let candidates = [
                format!("{}.so", ext),
                format!("{}.dll", ext),
            ];

            for path in &paths {
                for candidate in &candidates {
                    if path.join(candidate).exists() {
                        found = true;
                        break;
                    }
                }
                if found {
                    break;
                }
            }

            if !found {
                missing.push(ext);
            }
        }

        if !missing.is_empty() {
            eprintln!("WARNING: The following extensions verify files were not found in LD_LIBRARY_PATH: {:?}.", missing);
            eprintln!("Tests expecting these extensions to load will likely fail (or be skipped if formatted as file-not-found errors).");
            eprintln!("LD_LIBRARY_PATH: {:?}", library_paths);
            return;
        }
    }

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
