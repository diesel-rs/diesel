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

    #[derive(PartialEq, Eq, Debug)]
    struct UserWithoutId {
        name: String,
        age: Option<i16>,
    }

    // Compiler plugin will automatically invoke this based on schema
    table! {
        users {
            id -> Serial,
            name -> VarChar,
            age -> Nullable<SmallInt>,
        }
    }

    // Compiler plugin will replace this with #[derive(Queriable)]
    queriable! {
        User {
            id -> i32,
            name -> String,
            age -> Option<i16>,
        }
    }

    queriable! {
        UserWithoutId {
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
            (1, "Sean".to_string(), None::<i16>),
            (2, "Tess".to_string(), None::<i16>),
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
            User { id: 1, name: "Sean".to_string(), age: None },
            User { id: 2, name: "Tess".to_string(), age: None },
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
        // let select_id = users::table.select(posts::id);
        let select_name = users.select(name);
        let ids: Vec<_> = connection.query_all(&select_id)
            .unwrap().collect();
        let names: Vec<String> = connection.query_all(&select_name)
            .unwrap().collect();
        // This should fail type checking, and we should add a test to ensure
        // it continues to fail to compile.
        // let names: Vec<String> = connection.query_all(&select_id)
        // .unwrap().collect();

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
            ("Jim".to_string(), Some(30)),
            ("Bob".to_string(), Some(40)),
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
        connection.execute("INSERT INTO users (name, age) VALUES ('Jim', 30), ('Bob', 40)")
            .unwrap();

        let source = users.select((name, age));
        let expected_data = vec![
            UserWithoutId { name: "Jim".to_string(), age: Some(30) },
            UserWithoutId { name: "Bob".to_string(), age:  Some(40) },
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

        let select_count = unsafe { users::table.select_sql::<types::BigInt>("COUNT(*)") };
        let get_count = || connection.query_one::<_, i64>(&select_count).unwrap();
        // This should fail type checking, and we should add a test to ensure
        // it continues to fail to compile.
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
