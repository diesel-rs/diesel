#![allow(missing_docs)] // test only module
extern crate dotenvy;

use crate::prelude::*;

cfg_if! {
    if #[cfg(feature = "__sqlite-shared")] {
        pub type TestConnection = SqliteConnection;

        pub fn connection() -> TestConnection {
            SqliteConnection::establish(":memory:").unwrap()
        }

        pub fn database_url() -> String {
            String::from(":memory:")
        }
    } else if #[cfg(feature = "postgres")] {
        pub type TestConnection = PgConnection;

        pub fn connection() -> TestConnection {
            pg_connection()
        }

        pub fn database_url() -> String {
            pg_database_url()
        }
    } else if #[cfg(feature = "mysql")] {
        pub type TestConnection = MysqlConnection;

        pub fn connection() -> TestConnection {
            let mut conn = connection_no_transaction();
            conn.begin_test_transaction().unwrap();
            conn
        }

        pub fn connection_no_transaction() -> TestConnection {
            MysqlConnection::establish(&database_url()).unwrap()
        }

        pub fn database_url() -> String {
            dotenvy::var("MYSQL_UNIT_TEST_DATABASE_URL")
                .or_else(|_| dotenvy::var("DATABASE_URL"))
                .expect("DATABASE_URL must be set in order to run tests")
        }
    } else if #[cfg(feature = "mariadb")] {
        pub type TestConnection = MariadbConnection;

        pub fn connection() -> TestConnection {
            let mut conn = connection_no_transaction();
            conn.begin_test_transaction().unwrap();
            conn
        }

        pub fn connection_no_transaction() -> TestConnection {
            MariadbConnection::establish(&database_url()).unwrap()
        }

        pub fn database_url() -> String {
            dotenvy::var("MARIADB_UNIT_TEST_DATABASE_URL")
                .or_else(|_| dotenvy::var("DATABASE_URL"))
                .expect("DATABASE_URL must be set in order to run tests")
        }
    } else {
        compile_error!(
            "At least one backend must be used to test this crate.\n \
            Pass argument `--features \"<backend>\"` with one or more of the following backends, \
            'mysql', 'mariadb', 'postgres', or 'sqlite'. \n\n \
            ex. cargo test --features \"mysql mariadb postgres sqlite\"\n"
        );
    }
}

#[cfg(feature = "postgres")]
pub fn pg_connection() -> PgConnection {
    let mut conn = pg_connection_no_transaction();
    conn.begin_test_transaction().unwrap();
    conn
}

#[cfg(feature = "postgres")]
pub fn pg_connection_no_transaction() -> PgConnection {
    PgConnection::establish(&pg_database_url()).unwrap()
}

#[cfg(feature = "postgres")]
pub fn pg_database_url() -> String {
    dotenvy::var("PG_DATABASE_URL")
        .or_else(|_| dotenvy::var("DATABASE_URL"))
        .expect("DATABASE_URL must be set in order to run tests")
}

/// Asserts that every float written as the json type `ST` for the backend
/// `DB` reads back with identical bits, which needs `serde_json`'s
/// `float_roundtrip` feature.
#[cfg(all(
    feature = "serde_json",
    any(feature = "postgres_backend", feature = "mysql")
))]
pub(crate) fn assert_floats_survive_a_json_round_trip<ST, DB>(
    read: impl Fn(&[u8]) -> crate::deserialize::Result<serde_json::Value>,
) where
    ST: crate::sql_types::SqlType,
    DB: crate::backend::Backend + crate::sql_types::TypeMetadata,
    for<'a> DB::BindCollector<'a>: crate::query_builder::bind_collector::BindCollector<
            'a,
            DB,
            Buffer = crate::query_builder::bind_collector::ByteWrapper<'a>,
        >,
    DB::MetadataLookup: 'static,
    serde_json::Value: crate::serialize::ToSql<ST, DB>,
{
    use crate::query_builder::bind_collector::ByteWrapper;

    for float in [
        8.829872855928286e-308f64,
        -0.20221894534048165,
        1.7383394626966921e-307,
        6.178787134922198e305,
        0.1,
        f64::MIN_POSITIVE,
        f64::MAX,
    ] {
        let value = serde_json::Value::from(float);
        let mut buffer = Vec::new();
        {
            let mut out = crate::serialize::Output::test(ByteWrapper(&mut buffer));
            crate::serialize::ToSql::<ST, DB>::to_sql(&value, &mut out).unwrap();
        }
        let back = read(&buffer).unwrap();
        assert_eq!(
            back.as_f64().map(f64::to_bits),
            Some(float.to_bits()),
            "{float:?} came back as {back}"
        );
    }
}
