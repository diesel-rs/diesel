extern crate dotenv;

use diesel::*;
use self::dotenv::dotenv;

fn connection_no_data() -> Connection {
    dotenv().ok();

    let connection_url = ::std::env::var("DATABASE_URL").ok()
        .expect("DATABASE_URL must be set in order to run tests");
    let connection = Connection::establish(&connection_url).unwrap();
    connection.begin_test_transaction().unwrap();

    connection
}

fn establish_connection() -> Connection {
    let connection = connection_no_data();

    connection.execute("CREATE TABLE users (
        id SERIAL PRIMARY KEY,
        name VARCHAR NOT NULL
    )").unwrap();
    connection.execute("INSERT INTO users (name) VALUES ('Sean'), ('Tess')").unwrap();

    connection
}
