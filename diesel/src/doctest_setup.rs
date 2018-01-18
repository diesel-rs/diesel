extern crate dotenv;
#[macro_use] extern crate cfg_if;

use diesel::prelude::*;
use self::dotenv::dotenv;

cfg_if! {
    if #[cfg(feature = "postgres")] {
        #[allow(dead_code)]
        type DB = diesel::pg::Pg;

        fn connection_no_transaction() -> PgConnection {
            let connection_url = database_url_from_env("PG_DATABASE_URL");
            PgConnection::establish(&connection_url).unwrap()
        }

        fn connection_no_data() -> PgConnection {
            let connection = connection_no_transaction();
            connection.begin_test_transaction().unwrap();
            connection.execute("DROP TABLE IF EXISTS users CASCADE").unwrap();
            connection.execute("DROP TABLE IF EXISTS animals CASCADE").unwrap();
            connection.execute("DROP TABLE IF EXISTS posts CASCADE").unwrap();
            connection.execute("DROP TABLE IF EXISTS comments CASCADE").unwrap();

            connection
        }

        #[allow(dead_code)]
        fn establish_connection() -> PgConnection {
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

            connection.execute("CREATE TABLE posts (
                id SERIAL PRIMARY KEY,
                user_id INTEGER NOT NULL,
                title VARCHAR NOT NULL
            )").unwrap();
            connection.execute("INSERT INTO posts (user_id, title) VALUES
                (1, 'My first post'),
                (1, 'About Rust'),
                (2, 'My first post too')").unwrap();

            connection.execute("CREATE TABLE comments (
                id SERIAL PRIMARY KEY,
                post_id INTEGER NOT NULL,
                body VARCHAR NOT NULL
            )").unwrap();
            connection.execute("INSERT INTO comments (post_id, body) VALUES
                (1, 'Great post'),
                (2, 'Yay! I am learning Rust'),
                (3, 'I enjoyed your post')").unwrap();

            connection
        }
    } else if #[cfg(feature = "sqlite")] {
        #[allow(dead_code)]
        type DB = diesel::sqlite::Sqlite;

        fn connection_no_data() -> SqliteConnection {
            SqliteConnection::establish(":memory:").unwrap()
        }

        #[allow(dead_code)]
        fn establish_connection() -> SqliteConnection {
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

            connection.execute("CREATE TABLE posts (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                user_id INTEGER NOT NULL,
                title VARCHAR NOT NULL
            )").unwrap();
            connection.execute("INSERT INTO posts (user_id, title) VALUES
                (1, 'My first post'),
                (1, 'About Rust'),
                (2, 'My first post too')").unwrap();
            
            connection.execute("CREATE TABLE comments (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                post_id INTEGER NOT NULL,
                body VARCHAR NOT NULL
            )").unwrap();
            connection.execute("INSERT INTO comments (post_id, body) VALUES
                (1, 'Great post'),
                (2, 'Yay! I am learning Rust'),
                (3, 'I enjoyed your post')").unwrap();

            connection
        }
    } else if #[cfg(feature = "mysql")] {
        #[allow(dead_code)]
        type DB = diesel::mysql::Mysql;

        fn connection_no_data() -> MysqlConnection {
            let connection_url = database_url_from_env("MYSQL_UNIT_TEST_DATABASE_URL");
            let connection = MysqlConnection::establish(&connection_url).unwrap();
            connection.execute("DROP TABLE IF EXISTS users CASCADE").unwrap();
            connection.execute("DROP TABLE IF EXISTS animals CASCADE").unwrap();
            connection.execute("DROP TABLE IF EXISTS posts CASCADE").unwrap();
            connection.execute("DROP TABLE IF EXISTS comments CASCADE").unwrap();

            connection
        }

        #[allow(dead_code)]
        fn establish_connection() -> MysqlConnection {
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

            connection.execute("CREATE TABLE posts (
                id INTEGER PRIMARY KEY AUTO_INCREMENT,
                user_id INTEGER NOT NULL,
                title TEXT NOT NULL
            ) CHARACTER SET utf8mb4").unwrap();
            connection.execute("INSERT INTO posts (user_id, title) VALUES
                (1, 'My first post'),
                (1, 'About Rust'),
                (2, 'My first post too')").unwrap();
            
            connection.execute("CREATE TABLE comments (
                id INTEGER PRIMARY KEY AUTO_INCREMENT,
                post_id INTEGER NOT NULL,
                body TEXT NOT NULL
            ) CHARACTER SET utf8mb4").unwrap();
            connection.execute("INSERT INTO comments (post_id, body) VALUES
                (1, 'Great post'),
                (2, 'Yay! I am learning Rust'),
                (3, 'I enjoyed your post')").unwrap();

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

    joinable!(posts -> users (user_id));
    allow_tables_to_appear_in_same_query!(animals, comments, posts, users);
}
