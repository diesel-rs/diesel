#![deny(warnings)]
pub mod persistable;
pub mod types;

mod connection;
mod db_result;
mod query_source;
mod result;
mod row;

#[macro_use]
mod macros;

pub use result::*;
pub use query_source::{QuerySource, Queriable, Table, Column, JoinTo};
pub use connection::Connection;

#[cfg(test)]
mod test_usage_without_compiler_plugins {
    pub use super::*;
    use types::NativeSqlType;

    #[derive(PartialEq, Eq, Debug)]
    struct User {
        id: i32,
        name: String,
        hair_color: Option<String>,
    }

    impl User {
        fn new(id: i32, name: &str) -> Self {
            User { id: id, name: name.to_string(), hair_color: None }
        }
    }

    #[derive(PartialEq, Eq, Debug)]
    struct UserWithoutId {
        name: String,
        hair_color: Option<String>,
    }

    #[derive(PartialEq, Eq, Debug)]
    struct Post {
        id: i32,
        user_id: i32,
        title: String,
    }

    // Compiler plugin will automatically invoke this based on schema
    table! {
        users {
            id -> Serial,
            name -> VarChar,
            hair_color -> Nullable<VarChar>,
        }
    }

    table! {
        posts {
            id -> Serial,
            user_id -> Integer,
            title -> VarChar,
        }
    }

    // Compiler plugin will replace this with #[derive(Queriable)]
    queriable! {
        User {
            id -> i32,
            name -> String,
            hair_color -> Option<String>,
        }
    }

    queriable! {
        UserWithoutId {
            name -> String,
            hair_color -> Option<String>,
        }
    }

    queriable! {
        Post {
            id -> i32,
            user_id -> i32,
            title -> String,
        }
    }

    joinable!(posts -> users (user_id = id));
    belongs_to!(User, users, Post, posts);

    #[test]
    fn selecting_basic_data() {
        let connection = connection();
        setup_users_table(&connection);
        connection.execute("INSERT INTO users (name) VALUES ('Sean'), ('Tess')")
            .unwrap();

        let expected_data = vec![
            (1, "Sean".to_string(), None::<String>),
            (2, "Tess".to_string(), None::<String>),
         ];
        let actual_data: Vec<_> = connection.query_all(&users::table)
            .unwrap().collect();
        assert_eq!(expected_data, actual_data);
    }

    #[test]
    fn selecting_a_struct() {
        let connection = connection();
        setup_users_table(&connection);
        connection.execute("INSERT INTO users (name) VALUES ('Sean'), ('Tess')")
            .unwrap();

        let expected_users = vec![
            User::new(1, "Sean"),
            User::new(2, "Tess"),
        ];
        let actual_users: Vec<_> = connection.query_all(&users::table)
            .unwrap().collect();
        assert_eq!(expected_users, actual_users);
    }

    #[test]
    fn with_safe_select() {
        use self::users::columns::*;
        use self::users::table as users;

        let connection = connection();
        setup_users_table(&connection);
        connection.execute("INSERT INTO users (name) VALUES ('Sean'), ('Tess')")
            .unwrap();

        let select_id = users.select(id);
        // This should fail type checking, and we should add a test to ensure
        // it continues to fail to compile.
        // let select_id = users.select(posts::id);
        let select_name = users.select(name);
        let ids: Vec<_> = connection.query_all(&select_id)
            .unwrap().collect();
        let names: Vec<String> = connection.query_all(&select_name)
            .unwrap().collect();
        // This should fail type checking, and we should add a test to ensure
        // it continues to fail to compile.
        // let names: Vec<String> = connection.query_all(&select_id)
        //     .unwrap().collect();

        assert_eq!(vec![1, 2], ids);
        assert_eq!(vec!["Sean".to_string(), "Tess".to_string()], names);
    }

    #[test]
    fn selecting_multiple_columns() {
        use self::users::columns::*;
        use self::users::table as users;

        let connection = connection();
        setup_users_table(&connection);
        connection.execute("INSERT INTO users (name, hair_color) VALUES ('Jim', 'Black'), ('Bob', 'Brown')")
            .unwrap();

        let source = users.select((name, hair_color));
        // This should fail type checking, and we should add a test to ensure
        // it continues to fail to compile.
        // let source = users.select((posts::title, posts::user_id));
        // let source = users.select((posts::title, name));
        let expected_data = vec![
            ("Jim".to_string(), Some("Black".to_string())),
            ("Bob".to_string(), Some("Brown".to_string())),
        ];
        let actual_data: Vec<_> = connection.query_all(&source)
            .unwrap().collect();

        assert_eq!(expected_data, actual_data);
    }

    #[test]
    fn selecting_multiple_columns_into_struct() {
        use self::users::columns::*;
        use self::users::table as users;

        let connection = connection();
        setup_users_table(&connection);
        connection.execute("INSERT INTO users (name, hair_color) VALUES ('Jim', 'Black'), ('Bob', 'Brown')")
            .unwrap();

        let source = users.select((name, hair_color));
        let expected_data = vec![
            UserWithoutId { name: "Jim".to_string(), hair_color: Some("Black".to_string()) },
            UserWithoutId { name: "Bob".to_string(), hair_color: Some("Brown".to_string()) },
        ];
        let actual_data: Vec<_> = connection.query_all(&source)
            .unwrap().collect();

        assert_eq!(expected_data, actual_data);
    }

    #[test]
    fn with_select_sql() {
        let connection = connection();
        setup_users_table(&connection);
        connection.execute("INSERT INTO users (name) VALUES ('Sean'), ('Tess')")
            .unwrap();

        let select_count = users::table.select_sql::<types::BigInt>("COUNT(*)");
        let get_count = || connection.query_one::<_, i64>(&select_count).unwrap();
        // This should fail type checking, and we should add a test to ensure
        // it continues to fail to compile.
        // let actual_count = connection.query_one::<_, String>(&select_count).unwrap();

        assert_eq!(Some(2), get_count());

        connection.execute("INSERT INTO users (name) VALUES ('Jim')")
            .unwrap();

        assert_eq!(Some(3), get_count());
    }

    #[test]
    fn belongs_to() {
        let connection = connection();
        setup_users_table(&connection);
        setup_posts_table(&connection);

        connection.execute("INSERT INTO users (name) VALUES ('Sean'), ('Tess')")
            .unwrap();
        connection.execute("INSERT INTO posts (user_id, title) VALUES
            (1, 'Hello'),
            (2, 'World')
        ").unwrap();

        let sean = User::new(1, "Sean");
        let tess = User::new(2, "Tess");
        let seans_post = Post { id: 1, user_id: 1, title: "Hello".to_string() };
        let tess_post = Post { id: 2, user_id: 2, title: "World".to_string() };

        let expected_data = vec![(seans_post, sean), (tess_post, tess)];
        let source = posts::table.inner_join(users::table);
        let actual_data: Vec<_> = connection.query_all(&source).unwrap().collect();

        assert_eq!(expected_data, actual_data);
    }

    #[test]
    fn select_single_from_join() {
        let connection = connection();
        setup_users_table(&connection);
        setup_posts_table(&connection);

        connection.execute("INSERT INTO users (name) VALUES ('Sean'), ('Tess')")
            .unwrap();
        connection.execute("INSERT INTO posts (user_id, title) VALUES
            (1, 'Hello'),
            (2, 'World')
        ").unwrap();

        let source = posts::table.inner_join(users::table);
        let select_name = source.select(users::name);
        let select_title = source.select(posts::title);

        let expected_names = vec!["Sean".to_string(), "Tess".to_string()];
        let actual_names: Vec<String> = connection.query_all(&select_name).unwrap().collect();

        assert_eq!(expected_names, actual_names);

        let expected_titles = vec!["Hello".to_string(), "World".to_string()];
        let actual_titles: Vec<String> = connection.query_all(&select_title).unwrap().collect();

        assert_eq!(expected_titles, actual_titles);
    }

    #[test]
    fn select_multiple_from_join() {
        let connection = connection();
        setup_users_table(&connection);
        setup_posts_table(&connection);

        connection.execute("INSERT INTO users (name) VALUES ('Sean'), ('Tess')")
            .unwrap();
        connection.execute("INSERT INTO posts (user_id, title) VALUES
            (1, 'Hello'),
            (2, 'World')
        ").unwrap();

        let source = posts::table.inner_join(users::table)
            .select((users::name, posts::title));

        let expected_data = vec![
            ("Sean".to_string(), "Hello".to_string()),
            ("Tess".to_string(), "World".to_string()),
        ];
        let actual_data: Vec<_> = connection.query_all(&source).unwrap().collect();

        assert_eq!(expected_data, actual_data);
    }

    #[test]
    fn select_only_one_side_of_join() {
        let connection = connection();
        setup_users_table(&connection);
        setup_posts_table(&connection);

        connection.execute("INSERT INTO users (name) VALUES ('Sean'), ('Tess')")
            .unwrap();
        connection.execute("INSERT INTO posts (user_id, title) VALUES (2, 'Hello')")
            .unwrap();

        let source = users::table.inner_join(posts::table).select(users::star);

        let expected_data = vec![User::new(2, "Tess")];
        let actual_data: Vec<_> = connection.query_all(&source).unwrap().collect();

        assert_eq!(expected_data, actual_data);
    }

    #[test]
    fn find() {
        use self::users::table as users;

        let connection = connection();
        setup_users_table(&connection);

        connection.execute("INSERT INTO users (name) VALUES ('Sean'), ('Tess')")
            .unwrap();

        assert_eq!(Ok(Some(User::new(1, "Sean"))), connection.find(&users, &1));
        assert_eq!(Ok(Some(User::new(2, "Tess"))), connection.find(&users, &2));
        assert_eq!(Ok(None::<User>), connection.find(&users, &3));
        // This should fail type checking, and we should add a test to ensure
        // it continues to fail to compile.
        // connection.find(&users, &"1").unwrap();
    }

    table! {
        users_with_name_pk (name) {
            name -> VarChar,
        }
    }

    #[test]
    fn find_with_non_serial_pk() {
        use self::users_with_name_pk::table as users;

        let connection = connection();
        connection.execute("CREATE TABLE users_with_name_pk (name VARCHAR PRIMARY KEY)")
            .unwrap();
        connection.execute("INSERT INTO users_with_name_pk (name) VALUES ('Sean'), ('Tess')")
            .unwrap();

        assert_eq!(Ok(Some("Sean".to_string())), connection.find(&users, &"Sean"));
        assert_eq!(Ok(Some("Tess".to_string())), connection.find(&users, &"Tess".to_string()));
        assert_eq!(Ok(None::<String>), connection.find(&users, &"Wibble"));
        // This should fail type checking, and we should add a test to ensure
        // it continues to fail to compile.
        // connection.find(&users, &1).unwrap();
    }

    fn connection() -> Connection {
        let connection_url = ::std::env::var("DATABASE_URL").ok()
            .expect("DATABASE_URL must be set in order to run tests");
        let result = Connection::establish(&connection_url).unwrap();
        result.execute("BEGIN").unwrap();
        result
    }

    #[test]
    fn insert_records() {
        use self::users::table as users;
        let connection = connection();
        setup_users_table(&connection);

        let new_users = vec![
            NewUser::new("Sean", Some("Black")),
            NewUser::new("Tess", None),
        ];
        let inserted_users: Vec<_> = connection.insert(&users, new_users).unwrap().collect();

        let expected_users = vec![
            User { id: 1, name: "Sean".to_string(), hair_color: Some("Black".to_string()) },
            User { id: 2, name: "Tess".to_string(), hair_color: None },
        ];
        let actual_users: Vec<_> = connection.query_all(&users).unwrap().collect();

        assert_eq!(expected_users, actual_users);
        assert_eq!(expected_users, inserted_users);
    }

    #[test]
    fn insert_with_defaults() {
        use self::users::table as users;
        let connection = connection();
        connection.execute("CREATE TABLE users (
            id SERIAL PRIMARY KEY,
            name VARCHAR NOT NULL,
            hair_color VARCHAR NOT NULL DEFAULT 'Green'
        )").unwrap();
        let new_users = vec![
            NewUser::new("Sean", Some("Black")),
            NewUser::new("Tess", None),
        ];
        let inserted_users: Vec<_> = connection.insert(&users, new_users).unwrap().collect();

        let expected_users = vec![
            User { id: 1, name: "Sean".to_string(), hair_color: Some("Black".to_string()) },
            User { id: 2, name: "Tess".to_string(), hair_color: Some("Green".to_string()) },
        ];
        let actual_users: Vec<_> = connection.query_all(&users).unwrap().collect();

        assert_eq!(expected_users, actual_users);
        assert_eq!(expected_users, inserted_users);
    }

    #[test]
    fn insert_with_defaults_not_provided() {
        use self::users::table as users;
        let connection = connection();
        connection.execute("CREATE TABLE users (
            id SERIAL PRIMARY KEY,
            name VARCHAR NOT NULL,
            hair_color VARCHAR NOT NULL DEFAULT 'Green'
        )").unwrap();
        let new_users = vec![
            BaldUser { name: "Sean".to_string() },
            BaldUser { name: "Tess".to_string() },
        ];
        let inserted_users: Vec<_> = connection.insert(&users, new_users).unwrap().collect();

        let expected_users = vec![
            User { id: 1, name: "Sean".to_string(), hair_color: Some("Green".to_string()) },
            User { id: 2, name: "Tess".to_string(), hair_color: Some("Green".to_string()) },
        ];
        let actual_users: Vec<_> = connection.query_all(&users).unwrap().collect();

        assert_eq!(expected_users, actual_users);
        assert_eq!(expected_users, inserted_users);
    }

    struct NewUser {
        name: String,
        hair_color: Option<String>,
    }

    struct BaldUser {
        name: String,
    }

    impl NewUser {
        fn new(name: &str, hair_color: Option<&str>) -> Self {
            NewUser {
                name: name.to_string(),
                hair_color: hair_color.map(|s| s.to_string()),
            }
        }
    }

    insertable! {
        NewUser -> users {
            name -> String,
            hair_color -> Option<String>,
        }
    }

    insertable! {
        BaldUser -> users {
            name -> String,
        }
    }

    fn setup_users_table(connection: &Connection) {
        connection.execute("CREATE TABLE users (
            id SERIAL PRIMARY KEY,
            name VARCHAR NOT NULL,
            hair_color VARCHAR
        )").unwrap();
    }

    fn setup_posts_table(connection: &Connection) {
        connection.execute("CREATE TABLE posts (
            id SERIAL PRIMARY KEY,
            user_id INTEGER NOT NULL,
            title VARCHAR NOT NULL
        )").unwrap();
    }
}
