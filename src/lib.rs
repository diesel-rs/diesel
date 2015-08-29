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

    #[test]
    fn it_can_perform_a_basic_query() {
        let connection_url = env::var("DATABASE_URL").ok()
            .expect("DATABASE_URL must be set in order to run tests");
        let connection = Connection::establish(&connection_url)
            .unwrap();

        let _t = TestTable::new(&connection, "users",
            "(id SERIAL PRIMARY KEY, name VARCHAR NOT NULL)");
        connection.execute("INSERT INTO users (name) VALUES ('Sean'), ('Tess')")
            .unwrap();

        let expected_users = vec![
            User { id: 1, name: "Sean".to_string() },
            User { id: 2, name: "Tess".to_string() },
         ];
        let actual_users = connection.query_all(&UserTable).unwrap();
        assert_eq!(expected_users, actual_users);
    }

    struct TestTable<'a> {
        connection: &'a Connection,
        name: String,
    }

    impl<'a> Drop for TestTable<'a> {
        fn drop(&mut self) {
            self.connection.execute(&format!("DROP TABLE {}", &self.name))
                .unwrap();
        }
    }

    impl<'a> TestTable<'a> {
        fn new(connection: &'a Connection, name: &str, values: &str) -> Self {
            connection.execute(&format!("CREATE TABLE {} {}", name, values)).unwrap();
            TestTable {
                connection: connection,
                name: name.to_string(),
            }
        }
    }
}
