extern crate dotenv;

use diesel::*;

fn connection_no_data() -> Connection {
    let dotenv_path = ::std::env::current_dir()
        .and_then(|a| Ok(a.join("../.env"))).unwrap();
    dotenv::from_path(dotenv_path.as_path()).ok();

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
