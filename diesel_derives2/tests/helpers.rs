use diesel::prelude::*;
use std::sync::{Once, ONCE_INIT};

cfg_if! {
    if #[cfg(feature = "sqlite")] {
        embed_migrations!("../migrations/sqlite");

        pub type TestConnection = SqliteConnection;

        fn connection_no_transaction() -> TestConnection {
            SqliteConnection::establish(":memory:").unwrap()
        }
    } else if #[cfg(feature = "postgres")] {
        extern crate dotenv;

        embed_migrations!("../migrations/postgresql");

        pub type TestConnection = PgConnection;

        fn connection_no_transaction() -> TestConnection {
            let database_url = dotenv::var("PG_DATABASE_URL")
                .or_else(|_| dotenv::var("DATABASE_URL"))
                .expect("DATABASE_URL must be set in order to run tests");
            PgConnection::establish(&database_url).unwrap()
        }
    } else if #[cfg(feature = "mysql")] {
        extern crate dotenv;

        embed_migrations!("../migrations/mysql");

        pub type TestConnection = MysqlConnection;

        fn connection_no_transaction() -> TestConnection {
            let database_url = dotenv::var("MYSQL_DATABASE_URL")
                .or_else(|_| dotenv::var("DATABASE_URL"))
                .expect("DATABASE_URL must be set in order to run tests");
            MysqlConnection::establish(&database_url).unwrap()
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

static RUN_MIGRATIONS: Once = ONCE_INIT;

pub fn connection() -> TestConnection {
    let connection = connection_no_transaction();
    if cfg!(feature = "sqlite") {
        embedded_migrations::run(&connection).unwrap();
    } else {
        RUN_MIGRATIONS.call_once(|| {
            embedded_migrations::run(&connection).unwrap();
        })
    }
    connection.begin_test_transaction().unwrap();
    connection
}

pub fn connection_with_sean_and_tess_in_users_table() -> TestConnection {
    use schema::users::dsl::*;

    let connection = connection();
    ::diesel::insert_into(users)
        .values(&vec![
            (id.eq(1), name.eq("Sean"), hair_color.eq("black")),
            (id.eq(2), name.eq("Tess"), hair_color.eq("brown")),
        ])
        .execute(&connection)
        .unwrap();
    connection
}
