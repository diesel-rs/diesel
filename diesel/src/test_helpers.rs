#[cfg(feature = "sqlite")]
mod database_specific_helpers {
    use prelude::*;
    use sqlite::SqliteConnection;

    pub type TestConnection = SqliteConnection;

    pub fn connection() -> TestConnection {
        SqliteConnection::establish(":memory:").unwrap()
    }
}

#[cfg(all(feature = "postgres", not(feature = "sqlite")))]
mod database_specific_helpers {
    extern crate dotenv;

    use self::dotenv::dotenv;
    use std::env;

    use pg::PgConnection;
    use prelude::*;

    pub type TestConnection = PgConnection;

    pub fn connection() -> TestConnection {
        dotenv().ok();
        let database_url = env::var("DATABASE_URL")
            .expect("DATABASE_URL must be set to run tests");
        let conn = PgConnection::establish(&database_url).unwrap();
        conn.begin_test_transaction().unwrap();
        conn
    }
}

pub use self::database_specific_helpers::*;
