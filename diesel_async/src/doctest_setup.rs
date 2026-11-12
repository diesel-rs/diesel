#[allow(unused_imports)]
use diesel::prelude::{
    AsChangeset, ExpressionMethods, Identifiable, IntoSql, QueryDsl, QueryResult, Queryable,
    QueryableByName,
};

cfg_if::cfg_if! {
    if #[cfg(feature = "postgres")] {
        use diesel_async::AsyncPgConnection;
        #[allow(dead_code)]
        type DB = diesel::pg::Pg;
        #[allow(dead_code)]
        type DbConnection = AsyncPgConnection;

        fn database_url() -> String {
            database_url_from_env("PG_DATABASE_URL")
        }

        async fn connection_no_transaction() -> AsyncPgConnection {
            use diesel_async::AsyncConnection;
            let connection_url = database_url();
            AsyncPgConnection::establish(&connection_url).await.unwrap()
        }

        async fn connection_no_data() -> AsyncPgConnection {
            use diesel_async::AsyncConnection;
            let mut connection = connection_no_transaction().await;
            connection.begin_test_transaction().await.unwrap();
            connection
        }

        async fn create_tables(connection: &mut AsyncPgConnection) {
            use diesel_async::RunQueryDsl;
            diesel::sql_query("CREATE TEMPORARY TABLE users (
                id SERIAL PRIMARY KEY,
                name VARCHAR NOT NULL
            )").execute(connection).await.unwrap();
            diesel::sql_query(
                "INSERT INTO users (name) VALUES ('Sean'), ('Tess')"
            ).execute(connection).await.unwrap();

            diesel::sql_query(
                "CREATE TEMPORARY TABLE animals (
                id SERIAL PRIMARY KEY,
                species VARCHAR NOT NULL,
                legs INTEGER NOT NULL,
                name VARCHAR
            )").execute(connection).await.unwrap();
            diesel::sql_query(
                "INSERT INTO animals (species, legs, name) VALUES
                               ('dog', 4, 'Jack'),
                               ('spider', 8, null)"
            ).execute(connection)
             .await.unwrap();

            diesel::sql_query(
                "CREATE TEMPORARY TABLE posts (
                id SERIAL PRIMARY KEY,
                user_id INTEGER NOT NULL,
                title VARCHAR NOT NULL
            )").execute(connection).await.unwrap();
            diesel::sql_query(
                "INSERT INTO posts (user_id, title) VALUES
                (1, 'My first post'),
                (1, 'About Rust'),
                (2, 'My first post too')").execute(connection).await.unwrap();

            diesel::sql_query("CREATE TEMPORARY TABLE comments (
                id SERIAL PRIMARY KEY,
                post_id INTEGER NOT NULL,
                body VARCHAR NOT NULL
            )").execute(connection).await.unwrap();
            diesel::sql_query("INSERT INTO comments (post_id, body) VALUES
                (1, 'Great post'),
                (2, 'Yay! I am learning Rust'),
                (3, 'I enjoyed your post')").execute(connection).await.unwrap();

            diesel::sql_query("CREATE TEMPORARY TABLE brands (
                id SERIAL PRIMARY KEY,
                color VARCHAR NOT NULL DEFAULT 'Green',
                accent VARCHAR DEFAULT 'Blue'
            )").execute(connection).await.unwrap();
        }

        #[allow(dead_code)]
        async fn establish_connection() -> AsyncPgConnection {
            let mut connection = connection_no_data().await;
            create_tables(&mut connection).await;
            connection
        }
    }  else if #[cfg(feature = "mysql")] {
        use diesel_async::AsyncMysqlConnection;
        #[allow(dead_code)]
        type DB = diesel::mysql::Mysql;
        #[allow(dead_code)]
        type DbConnection = AsyncMysqlConnection;

        fn database_url() -> String {
            database_url_from_env("MYSQL_UNIT_TEST_DATABASE_URL")
        }

        async fn connection_no_data() -> AsyncMysqlConnection {
            use diesel_async::AsyncConnection;
            let connection_url = database_url();
            AsyncMysqlConnection::establish(&connection_url).await.unwrap()
        }

        async fn create_tables(connection: &mut AsyncMysqlConnection) {
            use diesel_async::RunQueryDsl;
            use diesel_async::AsyncConnection;
            diesel::sql_query("CREATE TEMPORARY TABLE IF NOT EXISTS users (
                id INTEGER PRIMARY KEY AUTO_INCREMENT,
                name TEXT NOT NULL
            ) CHARACTER SET utf8mb4").execute(connection).await.unwrap();


            diesel::sql_query("CREATE TEMPORARY TABLE IF NOT EXISTS animals (
                id INTEGER PRIMARY KEY AUTO_INCREMENT,
                species TEXT NOT NULL,
                legs INTEGER NOT NULL,
                name TEXT
            ) CHARACTER SET utf8mb4").execute(connection).await.unwrap();

            diesel::sql_query("CREATE TEMPORARY TABLE IF NOT EXISTS posts (
                id INTEGER PRIMARY KEY AUTO_INCREMENT,
                user_id INTEGER NOT NULL,
                title TEXT NOT NULL
            ) CHARACTER SET utf8mb4").execute(connection).await.unwrap();

            diesel::sql_query("CREATE TEMPORARY TABLE IF NOT EXISTS comments (
                id INTEGER PRIMARY KEY AUTO_INCREMENT,
                post_id INTEGER NOT NULL,
                body TEXT NOT NULL
            ) CHARACTER SET utf8mb4").execute(connection).await.unwrap();
            diesel::sql_query("CREATE TEMPORARY TABLE IF NOT EXISTS brands (
                id INTEGER PRIMARY KEY AUTO_INCREMENT,
                color VARCHAR(255) NOT NULL DEFAULT 'Green',
                accent VARCHAR(255) DEFAULT 'Blue'
            )").execute(connection).await.unwrap();

            connection.begin_test_transaction().await.unwrap();
            diesel::sql_query("INSERT INTO users (name) VALUES ('Sean'), ('Tess')").execute(connection).await.unwrap();
            diesel::sql_query("INSERT INTO posts (user_id, title) VALUES
                (1, 'My first post'),
                (1, 'About Rust'),
                (2, 'My first post too')").execute(connection).await.unwrap();
            diesel::sql_query("INSERT INTO comments (post_id, body) VALUES
                (1, 'Great post'),
                (2, 'Yay! I am learning Rust'),
                (3, 'I enjoyed your post')").execute(connection).await.unwrap();
            diesel::sql_query("INSERT INTO animals (species, legs, name) VALUES
                               ('dog', 4, 'Jack'),
                               ('spider', 8, null)").execute(connection).await.unwrap();

        }

        #[allow(dead_code)]
        async fn establish_connection() -> AsyncMysqlConnection {
            let mut connection = connection_no_data().await;
            create_tables(&mut connection).await;


            connection
        }
    }  else if #[cfg(feature = "sqlite")] {
        use diesel_async::sync_connection_wrapper::SyncConnectionWrapper;
        use diesel::sqlite::SqliteConnection;
        #[allow(dead_code)]
        type DB = diesel::sqlite::Sqlite;
        #[allow(dead_code)]
        type DbConnection = SyncConnectionWrapper<SqliteConnection>;

        fn database_url() -> String {
            database_url_from_env("SQLITE_DATABASE_URL")
        }

        async fn connection_no_data() -> SyncConnectionWrapper<SqliteConnection> {
            use diesel_async::AsyncConnection;
            let connection_url = database_url();
            SyncConnectionWrapper::<SqliteConnection>::establish(&connection_url).await.unwrap()
        }

        async fn create_tables(connection: &mut SyncConnectionWrapper<SqliteConnection>) {
            use diesel_async::RunQueryDsl;
            use diesel_async::AsyncConnection;
            diesel::sql_query("CREATE TEMPORARY TABLE IF NOT EXISTS users (
                id INTEGER PRIMARY KEY,
                name TEXT NOT NULL
            )").execute(connection).await.unwrap();


            diesel::sql_query("CREATE TEMPORARY TABLE IF NOT EXISTS animals (
                id INTEGER PRIMARY KEY,
                species TEXT NOT NULL,
                legs INTEGER NOT NULL,
                name TEXT
            )").execute(connection).await.unwrap();

            diesel::sql_query("CREATE TEMPORARY TABLE IF NOT EXISTS posts (
                id INTEGER PRIMARY KEY,
                user_id INTEGER NOT NULL,
                title TEXT NOT NULL
            )").execute(connection).await.unwrap();

            diesel::sql_query("CREATE TEMPORARY TABLE IF NOT EXISTS comments (
                id INTEGER PRIMARY KEY,
                post_id INTEGER NOT NULL,
                body TEXT NOT NULL
            )").execute(connection).await.unwrap();
            diesel::sql_query("CREATE TEMPORARY TABLE IF NOT EXISTS brands (
                id INTEGER PRIMARY KEY,
                color VARCHAR(255) NOT NULL DEFAULT 'Green',
                accent VARCHAR(255) DEFAULT 'Blue'
            )").execute(connection).await.unwrap();

            diesel::sql_query("INSERT INTO users (name) VALUES ('Sean'), ('Tess')").execute(connection).await.unwrap();
            diesel::sql_query("INSERT INTO posts (user_id, title) VALUES
                (1, 'My first post'),
                (1, 'About Rust'),
                (2, 'My first post too')").execute(connection).await.unwrap();
            diesel::sql_query("INSERT INTO comments (post_id, body) VALUES
                (1, 'Great post'),
                (2, 'Yay! I am learning Rust'),
                (3, 'I enjoyed your post')").execute(connection).await.unwrap();
            diesel::sql_query("INSERT INTO animals (species, legs, name) VALUES
                               ('dog', 4, 'Jack'),
                               ('spider', 8, null)").execute(connection).await.unwrap();

        }

        #[allow(dead_code)]
        async fn establish_connection() -> SyncConnectionWrapper<SqliteConnection> {
            use diesel_async::AsyncConnection;

            let mut connection = connection_no_data().await;
            connection.begin_test_transaction().await.unwrap();
            create_tables(&mut connection).await;
            connection
        }

        async fn connection_no_transaction() -> SyncConnectionWrapper<SqliteConnection> {
            use diesel_async::AsyncConnection;

            let mut connection = SyncConnectionWrapper::<SqliteConnection>::establish(":memory:").await.unwrap();
            create_tables(&mut connection).await;
            connection
        }

    } else if #[cfg(feature = "mariadb")] {
        use diesel_async::AsyncMariadbConnection;
        #[allow(dead_code)]
        type DB = diesel::mariadb::Mariadb;
        #[allow(dead_code)]
        type DbConnection = AsyncMariadbConnection;

        fn database_url() -> String {
            database_url_from_env("MARIADB_UNIT_TEST_DATABASE_URL")
        }

        async fn connection_no_data() -> AsyncMariadbConnection {
            use diesel_async::AsyncConnection;
            let connection_url = database_url();
            AsyncMariadbConnection::establish(&connection_url).await.unwrap()
        }

        async fn create_tables(connection: &mut AsyncMariadbConnection) {
            use diesel_async::RunQueryDsl;
            use diesel_async::AsyncConnection;
            diesel::sql_query("CREATE TEMPORARY TABLE IF NOT EXISTS users (
                id INTEGER PRIMARY KEY AUTO_INCREMENT,
                name TEXT NOT NULL
            ) CHARACTER SET utf8mb4").execute(connection).await.unwrap();


            diesel::sql_query("CREATE TEMPORARY TABLE IF NOT EXISTS animals (
                id INTEGER PRIMARY KEY AUTO_INCREMENT,
                species TEXT NOT NULL,
                legs INTEGER NOT NULL,
                name TEXT
            ) CHARACTER SET utf8mb4").execute(connection).await.unwrap();

            diesel::sql_query("CREATE TEMPORARY TABLE IF NOT EXISTS posts (
                id INTEGER PRIMARY KEY AUTO_INCREMENT,
                user_id INTEGER NOT NULL,
                title TEXT NOT NULL
            ) CHARACTER SET utf8mb4").execute(connection).await.unwrap();

            diesel::sql_query("CREATE TEMPORARY TABLE IF NOT EXISTS comments (
                id INTEGER PRIMARY KEY AUTO_INCREMENT,
                post_id INTEGER NOT NULL,
                body TEXT NOT NULL
            ) CHARACTER SET utf8mb4").execute(connection).await.unwrap();
            diesel::sql_query("CREATE TEMPORARY TABLE IF NOT EXISTS brands (
                id INTEGER PRIMARY KEY AUTO_INCREMENT,
                color VARCHAR(255) NOT NULL DEFAULT 'Green',
                accent VARCHAR(255) DEFAULT 'Blue'
            )").execute(connection).await.unwrap();

            connection.begin_test_transaction().await.unwrap();
            diesel::sql_query("INSERT INTO users (name) VALUES ('Sean'), ('Tess')").execute(connection).await.unwrap();
            diesel::sql_query("INSERT INTO posts (user_id, title) VALUES
                (1, 'My first post'),
                (1, 'About Rust'),
                (2, 'My first post too')").execute(connection).await.unwrap();
            diesel::sql_query("INSERT INTO comments (post_id, body) VALUES
                (1, 'Great post'),
                (2, 'Yay! I am learning Rust'),
                (3, 'I enjoyed your post')").execute(connection).await.unwrap();
            diesel::sql_query("INSERT INTO animals (species, legs, name) VALUES
                               ('dog', 4, 'Jack'),
                               ('spider', 8, null)").execute(connection).await.unwrap();

        }

        #[allow(dead_code)]
        async fn establish_connection() -> AsyncMariadbConnection {
            let mut connection = connection_no_data().await;
            create_tables(&mut connection).await;


            connection
        }
    } else {
        compile_error!(
            "At least one backend must be used to test this crate.\n \
            Pass argument `--features \"<backend>\"` with one or more of the following backends, \
            'mysql', 'mariadb', 'postgres', or 'sqlite'. \n\n \
            ex. cargo test --features \"mysql mariadb postgres sqlite\"\n"
        );
    }
}

fn database_url_from_env(backend_specific_env_var: &str) -> String {
    use std::env;

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
