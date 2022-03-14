extern crate dotenvy;

use crate::prelude::*;

cfg_if! {
    if #[cfg(feature = "sqlite")] {
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
    } else {
        compile_error!(
            "At least one backend must be used to test this crate.\n \
            Pass argument `--features \"<backend>\"` with one or more of the following backends, \
            'mysql', 'postgres', or 'sqlite'. \n\n \
            ex. cargo test --features \"mysql postgres sqlite\"\n"
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
