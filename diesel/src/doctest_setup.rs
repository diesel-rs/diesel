use diesel::prelude::*;
use dotenvy::dotenv;

cfg_if::cfg_if! {
    if #[cfg(feature = "postgres")] {
        #[allow(dead_code)]
        type DB = diesel::pg::Pg;
        type DbConnection = PgConnection;

        fn database_url_for_env() -> String {
            database_url_from_env("PG_DATABASE_URL")
        }

        fn connection_no_transaction() -> PgConnection {
            let connection_url = database_url_for_env();
            PgConnection::establish(&connection_url).unwrap()
        }

        fn setup_database(connection: &mut PgConnection) {
            connection.begin_test_transaction().unwrap();
            clean_tables(connection);
            create_tables_with_data(connection);
        }

        fn clean_tables(connection: &mut PgConnection) {
            diesel::sql_query("DROP TABLE IF EXISTS users CASCADE").execute(connection).unwrap();
            diesel::sql_query("DROP TABLE IF EXISTS animals CASCADE").execute(connection).unwrap();
            diesel::sql_query("DROP TABLE IF EXISTS posts CASCADE").execute(connection).unwrap();
            diesel::sql_query("DROP TABLE IF EXISTS comments CASCADE").execute(connection).unwrap();
            diesel::sql_query("DROP TABLE IF EXISTS brands CASCADE").execute(connection).unwrap();
        }

        fn connection_no_data() -> PgConnection {
            let mut connection = connection_no_transaction();
            connection.begin_test_transaction().unwrap();
            clean_tables(&mut connection);
            connection
        }

        fn create_tables_with_data(connection: &mut PgConnection) {
            diesel::sql_query("CREATE TABLE users (
                id SERIAL PRIMARY KEY,
                name VARCHAR NOT NULL
            )")
                .execute(connection)
                .unwrap();
            diesel::sql_query("INSERT INTO users (name) VALUES ('Sean'), ('Tess')")
                .execute(connection)
                .unwrap();

            diesel::sql_query("CREATE TABLE animals (
                id SERIAL PRIMARY KEY,
                species VARCHAR NOT NULL,
                legs INTEGER NOT NULL,
                name VARCHAR
            )")
                .execute(connection)
                .unwrap();
            diesel::sql_query("INSERT INTO animals (species, legs, name) VALUES
                               ('dog', 4, 'Jack'),
                               ('spider', 8, null)"
            ).execute(connection).unwrap();

            diesel::sql_query("CREATE TABLE posts (
                id SERIAL PRIMARY KEY,
                user_id INTEGER NOT NULL,
                title VARCHAR NOT NULL
            )").execute(connection).unwrap();
            diesel::sql_query("INSERT INTO posts (user_id, title) VALUES
                (1, 'My first post'),
                (1, 'About Rust'),
                (2, 'My first post too')").execute(connection).unwrap();

            diesel::sql_query("CREATE TABLE comments (
                id SERIAL PRIMARY KEY,
                post_id INTEGER NOT NULL,
                body VARCHAR NOT NULL
            )").execute(connection).unwrap();
            diesel::sql_query("INSERT INTO comments (post_id, body) VALUES
                (1, 'Great post'),
                (2, 'Yay! I am learning Rust'),
                (3, 'I enjoyed your post')").execute(connection).unwrap();

            diesel::sql_query("CREATE TABLE brands (
                id SERIAL PRIMARY KEY,
                color VARCHAR NOT NULL DEFAULT 'Green',
                accent VARCHAR DEFAULT 'Blue'
            )").execute(connection).unwrap();
        }

        #[allow(dead_code)]
        fn establish_connection() -> PgConnection {
            let mut connection = connection_no_data();
            create_tables_with_data(&mut connection);
            connection
        }
    } else if #[cfg(feature = "sqlite")] {
        #[allow(dead_code)]
        type DB = diesel::sqlite::Sqlite;
        type DbConnection = SqliteConnection;

        fn database_url_for_env() -> String {
            String::from(":memory:")
        }

        fn connection_no_data() -> SqliteConnection {
            SqliteConnection::establish(":memory:").unwrap()
        }

        fn setup_database(connection: &mut SqliteConnection) {
            create_tables_with_data(connection);
        }

        fn create_tables_with_data(connection: &mut SqliteConnection) {
            diesel::sql_query("CREATE TABLE users (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                name VARCHAR NOT NULL
            )").execute(connection).unwrap();
            diesel::sql_query("INSERT INTO users (name) VALUES ('Sean'), ('Tess')")
                .execute(connection).unwrap();

            diesel::sql_query("CREATE TABLE animals (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                species VARCHAR NOT NULL,
                legs INTEGER NOT NULL,
                name VARCHAR
            )").execute(connection).unwrap();
            diesel::sql_query("INSERT INTO animals (species, legs, name) VALUES
                               ('dog', 4, 'Jack'),
                               ('spider', 8, null)"
            ).execute(connection).unwrap();

            diesel::sql_query("CREATE TABLE posts (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                user_id INTEGER NOT NULL,
                title VARCHAR NOT NULL
            )").execute(connection).unwrap();
            diesel::sql_query("INSERT INTO posts (user_id, title) VALUES
                (1, 'My first post'),
                (1, 'About Rust'),
                (2, 'My first post too')").execute(connection).unwrap();

            diesel::sql_query("CREATE TABLE comments (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                post_id INTEGER NOT NULL,
                body VARCHAR NOT NULL
            )").execute(connection).unwrap();
            diesel::sql_query("INSERT INTO comments (post_id, body) VALUES
                (1, 'Great post'),
                (2, 'Yay! I am learning Rust'),
                (3, 'I enjoyed your post')").execute(connection).unwrap();
        }

        #[allow(dead_code)]
        fn establish_connection() -> SqliteConnection {
            let mut connection = connection_no_data();
            create_tables_with_data(&mut connection);
            connection
        }
    } else if #[cfg(feature = "mysql")] {
        #[allow(dead_code)]
        type DB = diesel::mysql::Mysql;
        type DbConnection = MysqlConnection;

        fn database_url_for_env() -> String {
            database_url_from_env("MYSQL_UNIT_TEST_DATABASE_URL")
        }

        fn setup_database(connection: &mut MysqlConnection) {
            clean_tables(connection);
            create_tables_with_data(connection);
        }

        fn clean_tables(connection: &mut MysqlConnection) {
            diesel::sql_query("SET FOREIGN_KEY_CHECKS=0").execute(connection).unwrap();
            diesel::sql_query("DROP TABLE IF EXISTS users CASCADE").execute(connection).unwrap();
            diesel::sql_query("DROP TABLE IF EXISTS animals CASCADE").execute(connection).unwrap();
            diesel::sql_query("DROP TABLE IF EXISTS posts CASCADE").execute(connection).unwrap();
            diesel::sql_query("DROP TABLE IF EXISTS comments CASCADE").execute(connection).unwrap();
            diesel::sql_query("DROP TABLE IF EXISTS brands CASCADE").execute(connection).unwrap();
            diesel::sql_query("SET FOREIGN_KEY_CHECKS=1").execute(connection).unwrap();
        }

        fn connection_no_data() -> MysqlConnection {
            let connection_url = database_url_for_env();
            let mut connection = MysqlConnection::establish(&connection_url).unwrap();
            clean_tables(&mut connection);
            connection
        }

        fn create_tables_with_data(connection: &mut MysqlConnection) {
            diesel::sql_query("CREATE TABLE users (
                id INTEGER PRIMARY KEY AUTO_INCREMENT,
                name TEXT NOT NULL
            ) CHARACTER SET utf8mb4").execute(connection).unwrap();
            diesel::sql_query("INSERT INTO users (name) VALUES ('Sean'), ('Tess')")
                      .execute(connection).unwrap();

            diesel::sql_query("CREATE TABLE animals (
                id INTEGER PRIMARY KEY AUTO_INCREMENT,
                species TEXT NOT NULL,
                legs INTEGER NOT NULL,
                name TEXT
            ) CHARACTER SET utf8mb4").execute(connection).unwrap();
            diesel::sql_query("INSERT INTO animals (species, legs, name) VALUES
                               ('dog', 4, 'Jack'),
                               ('spider', 8, null)").execute(connection).unwrap();

            diesel::sql_query("CREATE TABLE posts (
                id INTEGER PRIMARY KEY AUTO_INCREMENT,
                user_id INTEGER NOT NULL,
                title TEXT NOT NULL
            ) CHARACTER SET utf8mb4").execute(connection).unwrap();
            diesel::sql_query("INSERT INTO posts (user_id, title) VALUES
                (1, 'My first post'),
                (1, 'About Rust'),
                (2, 'My first post too')").execute(connection).unwrap();

            diesel::sql_query("CREATE TABLE comments (
                id INTEGER PRIMARY KEY AUTO_INCREMENT,
                post_id INTEGER NOT NULL,
                body TEXT NOT NULL
            ) CHARACTER SET utf8mb4").execute(connection).unwrap();
            diesel::sql_query("INSERT INTO comments (post_id, body) VALUES
                (1, 'Great post'),
                (2, 'Yay! I am learning Rust'),
                (3, 'I enjoyed your post')").execute(connection).unwrap();

            diesel::sql_query("CREATE TABLE brands (
                id INTEGER PRIMARY KEY AUTO_INCREMENT,
                color VARCHAR(255) NOT NULL DEFAULT 'Green',
                accent VARCHAR(255) DEFAULT 'Blue'
            )").execute(connection).unwrap();
        }

        #[allow(dead_code)]
        fn establish_connection() -> MysqlConnection {
            let mut connection = connection_no_data();
            create_tables_with_data(&mut connection);
            connection.begin_test_transaction().unwrap();
            connection
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

fn database_url_from_env(backend_specific_env_var: &str) -> String {
    use std::env;

    dotenv().ok();

    env::var(backend_specific_env_var)
        .or_else(|_| env::var("DATABASE_URL"))
        .expect("DATABASE_URL must be set in order to run tests")
}

mod schema {
    use diesel::prelude::*;

    table! {
        animals {
            id -> Integer,
            species -> VarChar,
            legs -> Integer,
            name -> Nullable<VarChar>,
        }
    }

    table! {
        comments {
            id -> Integer,
            post_id -> Integer,
            body -> VarChar,
        }
    }

    table! {
        posts {
            id -> Integer,
            user_id -> Integer,
            title -> VarChar,
        }
    }

    table! {
        users {
            id -> Integer,
            name -> VarChar,
        }
    }

    #[cfg(not(feature = "sqlite"))]
    table! {
        brands {
            id -> Integer,
            color -> VarChar,
            accent -> Nullable<VarChar>,
        }
    }

    joinable!(posts -> users (user_id));
    allow_tables_to_appear_in_same_query!(animals, comments, posts, users);
}
