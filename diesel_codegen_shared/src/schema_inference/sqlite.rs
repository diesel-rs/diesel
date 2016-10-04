use diesel::*;
use diesel::expression::dsl::sql;
use diesel::sqlite::SqliteConnection;
use std::error::Error;

table! {
    sqlite_master (name) {
        name -> VarChar,
    }
}

pub fn load_table_names(connection: &SqliteConnection) -> Result<Vec<String>, Box<Error>> {
    use self::sqlite_master::dsl::*;

    sqlite_master.select(name)
        .filter(name.not_like("\\_\\_%").escape('\\'))
        .filter(name.not_like("sqlite%"))
        .filter(sql("type='table'"))
        .load(connection)
        .map_err(Into::into)
}

#[test]
fn load_table_names_returns_nothing_when_no_tables_exist() {
    let conn = SqliteConnection::establish(":memory:").unwrap();
    assert_eq!(Ok(vec![]), load_table_names(&conn));
}

#[test]
fn load_table_names_includes_tables_that_exist() {
    let conn = SqliteConnection::establish(":memory:").unwrap();
    conn.execute("CREATE TABLE users (id INTEGER PRIMARY KEY AUTOINCREMENT)").unwrap();
    let table_names = load_table_names(&conn).unwrap();
    assert!(table_names.contains(&String::from("users")));
}

#[test]
fn load_table_names_excludes_diesel_metadata_tables() {
    let conn = SqliteConnection::establish(":memory:").unwrap();
    conn.execute("CREATE TABLE __diesel_metadata (id INTEGER PRIMARY KEY AUTOINCREMENT)").unwrap();
    let table_names = load_table_names(&conn).unwrap();
    assert!(!table_names.contains(&String::from("__diesel_metadata")));
}

#[test]
fn load_table_names_excludes_sqlite_metadata_tables() {
    let conn = SqliteConnection::establish(":memory:").unwrap();
    conn.execute("CREATE TABLE __diesel_metadata (id INTEGER PRIMARY KEY AUTOINCREMENT)").unwrap();
    conn.execute("CREATE TABLE users (id INTEGER PRIMARY KEY AUTOINCREMENT)").unwrap();
    let table_names = load_table_names(&conn);
    assert_eq!(Ok(vec![String::from("users")]), table_names);
}

#[test]
fn load_table_names_excludes_views() {
    let conn = SqliteConnection::establish(":memory:").unwrap();
    conn.execute("CREATE TABLE users (id INTEGER PRIMARY KEY AUTOINCREMENT)").unwrap();
    conn.execute("CREATE VIEW answer AS SELECT 42").unwrap();
    let table_names = load_table_names(&conn);
    assert_eq!(Ok(vec![String::from("users")]), table_names);
}
