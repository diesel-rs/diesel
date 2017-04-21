extern crate dotenv;
#[macro_use] extern crate cfg_if;

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
            connection.execute("DROP TABLE IF EXISTS animals").unwrap();

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

            connection.execute("CREATE TABLE animals (
                id SERIAL PRIMARY KEY,
                species VARCHAR NOT NULL,
                legs INTEGER NOT NULL,
                name VARCHAR
            )").unwrap();
            connection.execute("INSERT INTO animals (species, legs, name) VALUES
                               ('dog', 4, 'Jack'),
                               ('spider', 8, null)").unwrap();

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

            connection.execute("CREATE TABLE animals (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                species VARCHAR NOT NULL,
                legs INTEGER NOT NULL,
                name VARCHAR
            )").unwrap();
            connection.execute("INSERT INTO animals (species, legs, name) VALUES
                               ('dog', 4, 'Jack'),
                               ('spider', 8, null)").unwrap();

            connection
        }
    } else if #[cfg(feature = "mysql")] {
        #[allow(dead_code)]
        type DB = diesel::mysql::Mysql;

        fn connection_no_data() -> diesel::mysql::MysqlConnection {
            let connection_url = database_url_from_env("MYSQL_UNIT_TEST_DATABASE_URL");
            let connection = diesel::mysql::MysqlConnection::establish(&connection_url).unwrap();
            connection.execute("DROP TABLE IF EXISTS users").unwrap();
            connection.execute("DROP TABLE IF EXISTS animals").unwrap();

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

            connection.execute("CREATE TABLE animals (
                id INTEGER PRIMARY KEY AUTO_INCREMENT,
                species TEXT NOT NULL,
                legs INTEGER NOT NULL,
                name TEXT
            ) CHARACTER SET utf8mb4").unwrap();
            connection.execute("INSERT INTO animals (species, legs, name) VALUES
                               ('dog', 4, 'Jack'),
                               ('spider', 8, null)").unwrap();

            connection.begin_test_transaction().unwrap();
            connection
        }
    } else {
        // FIXME: https://github.com/rust-lang/rfcs/pull/1695
        // compile_error!("At least one backend must be enabled to run tests");
    }
}

fn database_url_from_env(backend_specific_env_var: &str) -> String {
    use std::env;

    dotenv().ok();

    env::var(backend_specific_env_var)
        .or_else(|_| env::var("DATABASE_URL"))
        .expect("DATABASE_URL must be set in order to run tests")
}

#[derive(Clone)]
#[allow(dead_code)]
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

impl_Insertable! {
    (users)
    struct NewUser {
        name: String,
    }
}

table! {
    animals {
        id -> Integer,
        species -> VarChar,
        legs -> Integer,
        name -> Nullable<VarChar>,
    }
}

#[derive(Clone)]
#[allow(dead_code)]
struct NewAnimal {
    species: String,
    legs: i32,
    name: Option<String>,
}

impl NewAnimal {
    pub fn new(species: &str, legs: i32, name: Option<&str>) -> Self {
        NewAnimal {
            species: species.into(),
            legs: legs,
            name: name.map(|n| n.into()),
        }
    }
}

impl_Insertable! {
    (animals)
    struct NewAnimal {
        species: String,
        legs: i32,
        name: Option<String>,
    }
}
