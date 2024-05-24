use diesel::prelude::*;
use diesel::sql_query;

#[allow(dead_code)] // that's used in one of the compile tests
pub type TestBackend = <TestConnection as diesel::Connection>::Backend;

cfg_if! {
    if #[cfg(feature = "sqlite")] {
        pub type TestConnection = SqliteConnection;

        pub fn connection() -> TestConnection {
            let mut conn = SqliteConnection::establish(":memory:").unwrap();
            sql_query("CREATE TABLE users (\
                id INTEGER PRIMARY KEY AUTOINCREMENT, \
                name VARCHAR NOT NULL, \
                hair_color VARCHAR DEFAULT 'Green',
                type VARCHAR DEFAULT 'regular')")
                .execute(&mut conn)
                .unwrap();
            sql_query("CREATE TABLE users_ (\
                id INTEGER PRIMARY KEY AUTOINCREMENT, \
                name VARCHAR NOT NULL, \
                hair_color VARCHAR DEFAULT 'Green',
                type VARCHAR DEFAULT 'regular')")
                .execute(&mut conn)
                .unwrap();
            conn
        }
    } else if #[cfg(feature = "postgres")] {
        extern crate dotenvy;

        pub type TestConnection = PgConnection;

        pub fn connection() -> TestConnection {
            let database_url = dotenvy::var("PG_DATABASE_URL")
                .or_else(|_| dotenvy::var("DATABASE_URL"))
                .expect("DATABASE_URL must be set in order to run tests");
            let mut conn = PgConnection::establish(&database_url).unwrap();
            conn.begin_test_transaction().unwrap();
            sql_query("DROP TABLE IF EXISTS users CASCADE").execute(&mut conn).unwrap();
            sql_query("DROP TABLE IF EXISTS users_ CASCADE").execute(&mut conn).unwrap();
            sql_query("CREATE TABLE users (\
                id SERIAL PRIMARY KEY, \
                name VARCHAR NOT NULL, \
                hair_color VARCHAR DEFAULT 'Green',
                type VARCHAR DEFAULT 'regular')")
                .execute(&mut conn)
                .unwrap();
            sql_query("CREATE TABLE users_ (\
                id SERIAL PRIMARY KEY, \
                name VARCHAR NOT NULL, \
                hair_color VARCHAR DEFAULT 'Green',
                type VARCHAR DEFAULT 'regular')")
                .execute(&mut conn)
                .unwrap();
            conn
        }
    } else if #[cfg(feature = "mysql")] {
        extern crate dotenvy;

        pub type TestConnection = MysqlConnection;

        pub fn connection() -> TestConnection {
            let database_url = dotenvy::var("MYSQL_UNIT_TEST_DATABASE_URL")
                .or_else(|_| dotenvy::var("DATABASE_URL"))
                .expect("DATABASE_URL must be set in order to run tests");
            let mut conn = MysqlConnection::establish(&database_url).unwrap();
            sql_query("DROP TABLE IF EXISTS users CASCADE").execute(&mut conn).unwrap();
            sql_query("DROP TABLE IF EXISTS users_ CASCADE").execute(&mut conn).unwrap();
            sql_query("CREATE TABLE users (\
                id INTEGER PRIMARY KEY AUTO_INCREMENT, \
                name TEXT NOT NULL, \
                hair_color VARCHAR(255) DEFAULT 'Green',
                type VARCHAR(255) DEFAULT 'regular')")
                .execute(&mut conn)
                .unwrap();
            sql_query("CREATE TABLE users_ (\
                id INTEGER PRIMARY KEY AUTO_INCREMENT, \
                name TEXT NOT NULL, \
                hair_color VARCHAR(255) DEFAULT 'Green',
                type VARCHAR(255) DEFAULT 'regular')")
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
    use crate::schema::users::dsl::*;

    let mut connection = connection();
    diesel::insert_into(users)
        .values(&vec![
            (
                id.eq(1),
                name.eq("Sean"),
                hair_color.eq("black"),
                r#type.eq("regular"),
            ),
            (
                id.eq(2),
                name.eq("Tess"),
                hair_color.eq("brown"),
                r#type.eq("admin"),
            ),
        ])
        .execute(&mut connection)
        .unwrap();
    connection
}
