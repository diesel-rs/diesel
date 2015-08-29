pub mod types;

mod query_source;
mod connection;
mod result;

pub use result::*;
pub use query_source::{QuerySource, Queriable};
pub use connection::Connection;

#[cfg(test)]
mod test_usage_without_macros_or_plugins {
    use super::{types, QuerySource, Queriable, Connection};
    use std::env;

    #[derive(PartialEq, Eq, Debug)]
    struct User {
        id: i32,
        name: String,
    }

    struct UserTable;

    unsafe impl QuerySource for UserTable {
        type SqlType = (types::Serial, types::VarChar);

        fn select_clause(&self) -> &str {
            "*"
        }

        fn from_clause(&self) -> &str {
            "users"
        }
    }

    impl<QS> Queriable<QS> for User where
        QS: QuerySource,
        (i32, String): types::FromSql<QS::SqlType>,
    {
        type Row = (i32, String);

        fn build(row: (i32, String)) -> Self {
            User {
                id: row.0,
                name: row.1,
            }
        }
    }

    fn connection() -> Connection {
        let connection_url = env::var("DATABASE_URL").ok()
            .expect("DATABASE_URL must be set in order to run tests");
        let result = Connection::establish(&connection_url).unwrap();
        result.execute("BEGIN").unwrap();
        result
    }

    fn setup_users_table(connection: &Connection) {
        connection.execute("CREATE TABLE users (id SERIAL PRIMARY KEY, name VARCHAR NOT NULL)")
            .unwrap();
    }

    #[test]
    fn it_can_perform_a_basic_query() {
        let connection = connection();
        setup_users_table(&connection);
        connection.execute("INSERT INTO users (name) VALUES ('Sean'), ('Tess')")
            .unwrap();

        let expected_users = vec![
            User { id: 1, name: "Sean".to_string() },
            User { id: 2, name: "Tess".to_string() },
         ];
        let actual_users = connection.query_all(&UserTable).unwrap();
        assert_eq!(expected_users, actual_users);
    }

    #[test]
    fn with_select_clause() {
        let connection = connection();
        setup_users_table(&connection);
        connection.execute("INSERT INTO users (name) VALUES ('Sean'), ('Tess')")
            .unwrap();

        let select_id = unsafe { UserTable.select::<types::Serial>("id") };
        let select_name = unsafe { UserTable.select::<types::VarChar>("name") };

        let expected_ids = vec![1, 2];
        let expected_names = vec!["Sean".to_string(), "Tess".to_string()];
        let actual_ids = connection.query_all::<_, i32>(&select_id).unwrap();
        let actual_names = connection.query_all::<_, String>(&select_name).unwrap();

        assert_eq!(expected_ids, actual_ids);
        assert_eq!(expected_names, actual_names);
    }
}
