pub mod types;

mod query_source;
mod connection;
mod result;
#[macro_use]
mod macros;

pub use result::*;
pub use query_source::{QuerySource, Queriable, Table, Column};
pub use connection::Connection;

#[cfg(test)]
mod test_usage_without_compiler_plugins {
    use super::*;
    use types::NativeSqlType;

    #[derive(PartialEq, Eq, Debug)]
    struct User {
        id: i32,
        name: String,
        age: Option<i16>,
    }

    table! {
        users {
            id -> Serial,
            name -> VarChar,
            age -> Nullable<SmallInt>,
        }
    }

    queriable! {
        User {
            id -> i32,
            name -> String,
            age -> Option<i16>,
        }
    }

    #[test]
    fn selecting_basic_data() {
        let connection = connection();
        setup_users_table(&connection);
        connection.execute("INSERT INTO users (name) VALUES ('Sean'), ('Tess')")
            .unwrap();

        let expected_data = vec![
            (1i32, "Sean".to_string(), None::<i16>),
            (2i32, "Tess".to_string(), None::<i16>),
         ];
        let actual_data = connection.query_all(&users::table).unwrap();
        assert_eq!(expected_data, actual_data);
    }

    #[test]
    fn selecting_a_struct() {
        let connection = connection();
        setup_users_table(&connection);
        connection.execute("INSERT INTO users (name) VALUES ('Sean'), ('Tess')")
            .unwrap();

        let expected_users = vec![
            User { id: 1, name: "Sean".to_string(), age: None },
            User { id: 2, name: "Tess".to_string(), age: None },
         ];
        let actual_users = connection.query_all(&users::table).unwrap();
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
        // fails to compile (we should test this)
        // let select_id = users::table.select(posts::id);
        let select_name = users.select(name);
        let ids = connection.query_all(&select_id).unwrap();
        let names: Vec<String> = connection.query_all(&select_name).unwrap();
        // fails to compile (we should test this)
        // let names: Vec<String> = connection.query_all(&select_id).unwrap();

        assert_eq!(vec![1, 2], ids);
        assert_eq!(vec!["Sean".to_string(), "Tess".to_string()], names);
    }

    #[test]
    fn selecting_multiple_columns() {
        use self::users::columns::*;
        use self::users::table as users;

        let connection = connection();
        setup_users_table(&connection);
        connection.execute("INSERT INTO users (name, age) VALUES ('Jim', 30), ('Bob', 40)")
            .unwrap();

        let source = users.select((name, age));
        let expected_data = vec![
            ("Jim".to_string(), Some(30i16)),
            ("Bob".to_string(), Some(40i16)),
        ];
        let actual_data = connection.query_all(&source).unwrap();

        assert_eq!(expected_data, actual_data);
    }

    #[test]
    fn with_select_sql() {
        let connection = connection();
        setup_users_table(&connection);
        connection.execute("INSERT INTO users (name) VALUES ('Sean'), ('Tess')")
            .unwrap();

        let select_count = unsafe { users::table.select_sql::<types::BigInt>("COUNT(*)") };
        let get_count = || connection.query_one::<_, i64>(&select_count).unwrap();
        // fails to compile (we should test this)
        // let actual_count = connection.query_one::<_, String>(&select_count).unwrap();

        assert_eq!(Some(2), get_count());

        connection.execute("INSERT INTO users (name) VALUES ('Jim')")
            .unwrap();

        assert_eq!(Some(3), get_count());
    }

    fn connection() -> Connection {
        let connection_url = ::std::env::var("DATABASE_URL").ok()
            .expect("DATABASE_URL must be set in order to run tests");
        let result = Connection::establish(&connection_url).unwrap();
        result.execute("BEGIN").unwrap();
        result
    }

    fn setup_users_table(connection: &Connection) {
        connection.execute("CREATE TABLE users (
            id SERIAL PRIMARY KEY,
            name VARCHAR NOT NULL,
            age SMALLINT
        )").unwrap();
    }
}
