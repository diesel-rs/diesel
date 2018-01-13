use prelude::*;

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
        extern crate dotenv;

        pub type TestConnection = PgConnection;

        pub fn connection() -> TestConnection {
            let conn = PgConnection::establish(&database_url()).unwrap();
            conn.begin_test_transaction().unwrap();
            conn
        }

        pub fn database_url() -> String {
            dotenv::var("PG_DATABASE_URL")
                .or_else(|_| dotenv::var("DATABASE_URL"))
                .expect("DATABASE_URL must be set in order to run tests")
        }
    } else if #[cfg(feature = "mysql")] {
        extern crate dotenv;

        pub type TestConnection = MysqlConnection;

        pub fn connection() -> TestConnection {
            let conn = connection_no_transaction();
            conn.begin_test_transaction().unwrap();
            conn
        }

        pub fn connection_no_transaction() -> TestConnection {
            MysqlConnection::establish(&database_url()).unwrap()
        }

        fn database_url() -> String {
            dotenv::var("MYSQL_UNIT_TEST_DATABASE_URL")
                .or_else(|_| dotenv::var("DATABASE_URL"))
                .expect("DATABASE_URL must be set in order to run tests");
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
