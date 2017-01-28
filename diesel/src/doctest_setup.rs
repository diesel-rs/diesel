extern crate dotenv;

use diesel::prelude::*;
use self::dotenv::dotenv;

#[cfg(feature = "postgres")]
#[allow(dead_code)]
type DB = diesel::pg::Pg;

#[cfg(feature = "postgres")]
fn connection_no_data() -> diesel::pg::PgConnection {
    dotenv().ok();

    let connection_url = ::std::env::var("DATABASE_URL")
        .expect("DATABASE_URL must be set in order to run tests");
    let connection = diesel::pg::PgConnection::establish(&connection_url).unwrap();
    connection.begin_test_transaction().unwrap();
    connection.execute("DROP TABLE IF EXISTS users").unwrap();

    connection
}

#[cfg(feature = "postgres")]
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

#[cfg(all(not(feature = "postgres"), feature = "sqlite"))]
#[allow(dead_code)]
type DB = diesel::sqlite::Sqlite;

#[cfg(all(not(feature = "postgres"), feature = "sqlite"))]
fn connection_no_data() -> diesel::sqlite::SqliteConnection {
    diesel::sqlite::SqliteConnection::establish(":memory:").unwrap()
}

#[cfg(all(not(feature = "postgres"), feature = "sqlite"))]
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
