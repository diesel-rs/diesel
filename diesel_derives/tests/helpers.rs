use diesel::prelude::*;
use diesel::sql_query;

cfg_if! {
    if #[cfg(feature = "sqlite")] {
        pub type TestConnection = SqliteConnection;

        pub fn connection() -> TestConnection {
            let mut conn = SqliteConnection::establish(":memory:").unwrap();
            sql_query("CREATE TABLE users (\
                id INTEGER PRIMARY KEY AUTOINCREMENT, \
                name VARCHAR NOT NULL, \
                hair_color VARCHAR DEFAULT 'Green')")
                .execute(&mut conn)
                .unwrap();
            conn
        }
    } else if #[cfg(feature = "postgres")] {
        extern crate dotenv;

        pub type TestConnection = PgConnection;

        pub fn connection() -> TestConnection {
            let database_url = dotenv::var("PG_DATABASE_URL")
                .or_else(|_| dotenv::var("DATABASE_URL"))
                .expect("DATABASE_URL must be set in order to run tests");
            let mut conn = PgConnection::establish(&database_url).unwrap();
            conn.begin_test_transaction().unwrap();
            sql_query("DROP TABLE IF EXISTS users CASCADE").execute(&mut conn).unwrap();
            sql_query("CREATE TABLE users (\
                id SERIAL PRIMARY KEY, \
                name VARCHAR NOT NULL, \
                hair_color VARCHAR DEFAULT 'Green')")
                .execute(&mut conn)
                .unwrap();
            conn
        }
    } else if #[cfg(feature = "mysql")] {
        extern crate dotenv;

        pub type TestConnection = MysqlConnection;

        pub fn connection() -> TestConnection {
            let database_url = dotenv::var("MYSQL_UNIT_TEST_DATABASE_URL")
                .or_else(|_| dotenv::var("DATABASE_URL"))
                .expect("DATABASE_URL must be set in order to run tests");
            let mut conn = MysqlConnection::establish(&database_url).unwrap();
            sql_query("DROP TABLE IF EXISTS users CASCADE").execute(&mut conn).unwrap();
            sql_query("CREATE TABLE users (\
                id INTEGER PRIMARY KEY AUTO_INCREMENT, \
                name TEXT NOT NULL, \
                hair_color VARCHAR(255) DEFAULT 'Green')")
                .execute(&mut conn)
                .unwrap();
            conn.begin_test_transaction().unwrap();
            conn
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

pub fn connection_with_sean_and_tess_in_users_table() -> TestConnection {
    use schema::users::dsl::*;

    let mut connection = connection();
    ::diesel::insert_into(users)
        .values(&vec![
            (id.eq(1), name.eq("Sean"), hair_color.eq("black")),
            (id.eq(2), name.eq("Tess"), hair_color.eq("brown")),
        ])
        .execute(&mut connection)
        .unwrap();
    connection
}
