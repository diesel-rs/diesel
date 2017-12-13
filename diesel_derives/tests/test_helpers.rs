use diesel::prelude::*;

cfg_if! {
    if #[cfg(feature = "sqlite")] {
        pub type TestConnection = SqliteConnection;

        pub fn connection() -> TestConnection {
            SqliteConnection::establish(":memory:").unwrap()
        }
    } else if #[cfg(feature = "postgres")] {
        extern crate dotenv;

        use self::dotenv::dotenv;
        use std::env;

        pub type TestConnection = PgConnection;

        pub fn connection() -> TestConnection {
            dotenv().ok();
            let database_url = env::var("PG_DATABASE_URL")
                .or_else(|_| env::var("DATABASE_URL"))
                .expect("DATABASE_URL must be set in order to run tests");
            let conn = PgConnection::establish(&database_url).unwrap();
            conn.begin_test_transaction().unwrap();
            conn
        }
    } else if #[cfg(feature = "mysql")] {
        extern crate dotenv;

        use self::dotenv::dotenv;
        use std::env;

        pub type TestConnection = MysqlConnection;

        pub fn connection() -> TestConnection {
            let conn = connection_no_transaction();
            conn.begin_test_transaction().unwrap();
            conn
        }

        pub fn connection_no_transaction() -> TestConnection {
            dotenv().ok();
            let database_url = env::var("MYSQL_UNIT_TEST_DATABASE_URL")
                .or_else(|_| env::var("DATABASE_URL"))
                .expect("DATABASE_URL must be set in order to run tests");
            MysqlConnection::establish(&database_url).unwrap()
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
