use diesel::prelude::*;

cfg_if! {
    if #[cfg(feature = "sqlite")] {
        use diesel::sqlite::SqliteConnection;

        pub type TestConnection = SqliteConnection;

        pub fn connection() -> TestConnection {
            SqliteConnection::establish(":memory:").unwrap()
        }
    } else if #[cfg(feature = "postgres")] {
        extern crate dotenv;

        use self::dotenv::dotenv;
        use std::env;

        use diesel::pg::PgConnection;

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

        use diesel::mysql::MysqlConnection;

        pub type TestConnection = MysqlConnection;

        pub fn connection() -> TestConnection {
            let conn = connection_no_transaction();
            conn.begin_test_transaction().unwrap();
            conn
        }

        pub fn connection_no_transaction() -> TestConnection {
            dotenv().ok();
            let database_url = env::var("MYSQL_DATABASE_URL")
                .or_else(|_| env::var("DATABASE_URL"))
                .expect("DATABASE_URL must be set in order to run tests");
            MysqlConnection::establish(&database_url).unwrap()
        }
    } else {

        pub type TestConnection = ();
        pub fn connection() -> TestConnection {
            panic!("At least one backend must be enabled to run tests")
        }
        // FIXME: https://github.com/rust-lang/rfcs/pull/1695
        // compile_error!("At least one backend must be enabled to run tests");
    }
}
