extern crate dotenv;

use diesel::prelude::*;
use diesel::backend;
use self::dotenv::dotenv;

fn connection_no_data() -> diesel::connection::PgConnection {
    dotenv().ok();

    let connection_url = ::std::env::var("DATABASE_URL")
        .expect("DATABASE_URL must be set in order to run tests");
    let connection = diesel::connection::PgConnection::establish(&connection_url).unwrap();
    connection.begin_test_transaction().unwrap();
    connection.execute("DROP TABLE IF EXISTS users").unwrap();

    connection
}

fn establish_connection() -> diesel::connection::PgConnection {
    let connection = connection_no_data();

    connection.execute("CREATE TABLE users (
        id SERIAL PRIMARY KEY,
        name VARCHAR NOT NULL
    )").unwrap();
    connection.execute("INSERT INTO users (name) VALUES ('Sean'), ('Tess')").unwrap();

    connection
}
