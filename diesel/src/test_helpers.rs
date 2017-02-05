use prelude::*;

cfg_if! {
    if #[cfg(feature = "sqlite")] {
        use sqlite::SqliteConnection;

        pub type TestConnection = SqliteConnection;

        pub fn connection() -> TestConnection {
            SqliteConnection::establish(":memory:").unwrap()
        }
    } else if #[cfg(feature = "postgres")] {
        extern crate dotenv;

        use self::dotenv::dotenv;
        use std::env;

        use pg::PgConnection;

        pub type TestConnection = PgConnection;

        pub fn connection() -> TestConnection {
            dotenv().ok();
            let database_url = env::var("DATABASE_URL")
                .expect("DATABASE_URL must be set to run tests");
            let conn = PgConnection::establish(&database_url).unwrap();
            conn.begin_test_transaction().unwrap();
            conn
        }
    } else if #[cfg(feature = "mysql")] {
        extern crate dotenv;

        use self::dotenv::dotenv;
        use std::env;

        use mysql::MysqlConnection;

        pub type TestConnection = MysqlConnection;

        pub fn connection() -> TestConnection {
            dotenv().ok();
            let database_url = env::var("DATABASE_URL")
                .expect("DATABASE_URL must be set to run tests");
            let conn = MysqlConnection::establish(&database_url).unwrap();
            conn.begin_test_transaction().unwrap();
            conn
        }
    } else {
        // FIXME: https://github.com/rust-lang/rfcs/pull/1695
        // compile_error!("At least one backend must be enabled to run tests");
    }
}
