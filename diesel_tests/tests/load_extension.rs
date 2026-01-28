use crate::schema::connection_without_transaction;
use diesel::sql_types::Text;
use diesel::sqlite::{
    SqliteExtension, SqliteMathFunctionsExtension, SqliteSpellfix1Extension, SqliteUUIDExtension,
};
use diesel::{select, RunQueryDsl};

#[test]
fn test_load_all_extensions() {
    let mut conn = connection_without_transaction();
    let restricted_build = std::env::var("DIESEL_TEST_SQLITE_EXTENSIONS_DISABLED").is_ok();

    test_extension::<SqliteUUIDExtension, _>(&mut conn, restricted_build, "uuid", |conn| {
        let uuid_result = select(diesel::dsl::sql::<Text>("uuid()")).get_result::<String>(conn);
        assert!(uuid_result.is_ok(), "uuid() function should be available");
        let uuid_value = uuid_result.unwrap();
        assert_eq!(uuid_value.len(), 36);
    });

    test_extension::<SqliteMathFunctionsExtension, _>(
        &mut conn,
        restricted_build,
        "extension-functions",
        |conn| {
            let cos_result = select(diesel::dsl::sql::<diesel::sql_types::Double>("cos(0)"))
                .get_result::<f64>(conn);
            // If compiled with math functions, this works.
            if let Ok(val) = cos_result {
                assert!((val - 1.0).abs() < 1e-6);
            }
        },
    );

    test_extension::<SqliteSpellfix1Extension, _>(
        &mut conn,
        restricted_build,
        "spellfix1",
        |conn| {
            let dist = select(diesel::dsl::sql::<diesel::sql_types::Integer>(
                "editdist1('apple', 'apply')",
            ))
            .get_result::<i32>(conn);
            assert_eq!(dist, Ok(1), "spellfix1 should be loaded: editdist1 failed");
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
                assert_ne!(
                    error_string, "not authorized",
                    "Should have been authorized to load extension {}: {}",
                    name, error_string
                );
                assert_ne!(
                    error_string, "no such function: load_extension",
                    "load_extension function should be available for {}",
                    name
                );
            }
        }
    }
}
