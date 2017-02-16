extern crate dotenv;
#[macro_use] extern crate cfg_if;
#[macro_use] extern crate diesel_codegen;

use diesel::prelude::*;
use self::dotenv::dotenv;

cfg_if! {
    if #[cfg(feature = "postgres")] {
        #[allow(dead_code)]
        type DB = diesel::pg::Pg;

        fn connection_no_data() -> diesel::pg::PgConnection {
            let connection_url = database_url_from_env("PG_DATABASE_URL");
            let connection = diesel::pg::PgConnection::establish(&connection_url).unwrap();
            connection.begin_test_transaction().unwrap();
            connection.execute("DROP TABLE IF EXISTS users").unwrap();

            connection
        }

        #[allow(dead_code)]
        fn establish_connection() -> diesel::pg::PgConnection {
            let connection = connection_no_data();

            connection.execute("CREATE TABLE users (
                id SERIAL PRIMARY KEY,
                name VARCHAR NOT NULL
            )").unwrap();
            connection.execute("INSERT INTO users (name) VALUES ('Sean'), ('Tess')").unwrap();

            connection
        }
    } else if #[cfg(feature = "sqlite")] {
        #[allow(dead_code)]
        type DB = diesel::sqlite::Sqlite;

        fn connection_no_data() -> diesel::sqlite::SqliteConnection {
            diesel::sqlite::SqliteConnection::establish(":memory:").unwrap()
        }

        #[allow(dead_code)]
        fn establish_connection() -> diesel::sqlite::SqliteConnection {
            let connection = connection_no_data();

            connection.execute("CREATE TABLE users (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                name VARCHAR NOT NULL
            )").unwrap();
            connection.execute("INSERT INTO users (name) VALUES ('Sean'), ('Tess')").unwrap();

            connection
        }
    } else if #[cfg(feature = "mysql")] {
        #[allow(dead_code)]
        type DB = diesel::mysql::Mysql;

        fn connection_no_data() -> diesel::mysql::MysqlConnection {
            let connection_url = database_url_from_env("MYSQL_UNIT_TEST_DATABASE_URL");
            let connection = diesel::mysql::MysqlConnection::establish(&connection_url).unwrap();
            connection.execute("DROP TABLE IF EXISTS users").unwrap();

            connection
        }

        #[allow(dead_code)]
        fn establish_connection() -> diesel::mysql::MysqlConnection {
            let connection = connection_no_data();

            connection.execute("CREATE TABLE users (
                id INTEGER PRIMARY KEY AUTO_INCREMENT,
                name TEXT NOT NULL
            ) CHARACTER SET utf8mb4").unwrap();
            connection.execute("INSERT INTO users (name) VALUES ('Sean'), ('Tess')").unwrap();
            connection.begin_test_transaction().unwrap();

            connection
        }
    } else {
        // FIXME: https://github.com/rust-lang/rfcs/pull/1695
        // compile_error!("At least one backend must be enabled to run tests");
    }
}

fn database_url_from_env(bakend_specific_env_var: &str) -> String {
    use std::env;

    dotenv().ok();

    env::var(bakend_specific_env_var)
        .or_else(|_| env::var("DATABASE_URL"))
        .expect("DATABASE_URL must be set in order to run tests")
}

#[derive(Clone, Insertable)]
#[allow(dead_code)]
#[table_name = "users"]
struct NewUser {
    name: String,
}

impl NewUser {
    pub fn new(name: &str) -> Self {
        NewUser {
            name: name.into(),
        }
    }
}
