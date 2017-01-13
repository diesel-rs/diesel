use diesel::*;
use diesel::expression::dsl::sql;
use diesel::sqlite::SqliteConnection;
use std::error::Error;

use super::data_structures::*;

table! {
    sqlite_master (name) {
        name -> VarChar,
    }
}

table!{
    pragma_table_info (cid) {
        cid ->Integer,
        name -> VarChar,
        type_name -> VarChar,
        notnull -> Bool,
        dflt_value -> Nullable<VarChar>,
        pk -> Bool,
    }
}

pub fn load_table_names(connection: &SqliteConnection, schema_name: Option<&str>)
    -> Result<Vec<String>, Box<Error>>
{
    use self::sqlite_master::dsl::*;

    if !schema_name.is_none() {
        return Err("sqlite cannot infer schema for databases other than the \
                    main database".into());
    }

    sqlite_master.select(name)
        .filter(name.not_like("\\_\\_%").escape('\\'))
        .filter(name.not_like("sqlite%"))
        .filter(sql("type='table'"))
        .load(connection)
        .map_err(Into::into)
}

pub fn get_table_data(conn: &SqliteConnection, table_name: &str)
    -> QueryResult<Vec<ColumnInformation>>
{
    let query = format!("PRAGMA TABLE_INFO('{}')", table_name);
    sql::<pragma_table_info::SqlType>(&query).load(conn)
}

struct FullTableInfo {
    _cid: i32,
    name: String,
    _type_name: String,
    _not_null: bool,
    _dflt_value: Option<String>,
    primary_key: bool,
}

impl_Queryable! {
    struct FullTableInfo {
        _cid: i32,
        name: String,
        _type_name: String,
        _not_null: bool,
        _dflt_value: Option<String>,
        primary_key: bool,
    }
}

pub fn get_primary_keys(conn: &SqliteConnection, table_name: &str) -> QueryResult<Vec<String>> {
    let query = format!("PRAGMA TABLE_INFO('{}')", table_name);
    let results = try!(sql::<pragma_table_info::SqlType>(&query)
        .load::<FullTableInfo>(conn));
    Ok(results.iter()
        .filter_map(|i| if i.primary_key { Some(i.name.clone()) } else { None })
        .collect())
}

pub fn determine_column_type(attr: &ColumnInformation) -> Result<ColumnType, Box<Error>> {
    let type_name = attr.type_name.to_lowercase();
    let path = if is_bool(&type_name) {
        vec!["diesel".into(), "types".into(), "Bool".into()]
    } else if is_smallint(&type_name) {
        vec!["diesel".into(), "types".into(), "SmallInt".into()]
    } else if is_bigint(&type_name) {
        vec!["diesel".into(), "types".into(), "BigInt".into()]
    } else if type_name.contains("int") {
        vec!["diesel".into(), "types".into(), "Integer".into()]
    } else if is_text(&type_name) {
        vec!["diesel".into(), "types".into(), "Text".into()]
    } else if type_name.contains("blob") || type_name.is_empty() {
        vec!["diesel".into(), "types".into(), "Binary".into()]
    } else if is_float(&type_name) {
        vec!["diesel".into(), "types".into(), "Float".into()]
    } else if is_double(&type_name) {
        vec!["diesel".into(), "types".into(), "Double".into()]
    } else {
        return Err(format!("Unsupported type: {}", type_name).into())
    };

    Ok(ColumnType {
        path: path,
        is_array: false,
        is_nullable: attr.nullable,
    })
}

fn is_text(type_name: &str) -> bool {
    type_name.contains("char") ||
    type_name.contains("clob") ||
        type_name.contains("text")
}

fn is_bool(type_name: &str) -> bool {
    type_name == "boolean" ||
        type_name.contains("tiny") &&
        type_name.contains("int")
}

fn is_smallint(type_name: &str) -> bool {
    type_name == "int2" ||
        type_name.contains("small") &&
        type_name.contains("int")
}

fn is_bigint(type_name: &str) -> bool {
    type_name == "int8" ||
        type_name.contains("big") &&
        type_name.contains("int")
}

fn is_float(type_name: &str) -> bool {
    type_name.contains("float") ||
        type_name.contains("real")
}

fn is_double(type_name: &str) -> bool {
    type_name.contains("double") ||
        type_name.contains("num") ||
        type_name.contains("dec")
}

#[test]
fn load_table_names_returns_nothing_when_no_tables_exist() {
    let conn = SqliteConnection::establish(":memory:").unwrap();
    assert_eq!(Vec::<String>::new(), load_table_names(&conn, None).unwrap());
}

#[test]
fn load_table_names_includes_tables_that_exist() {
    let conn = SqliteConnection::establish(":memory:").unwrap();
    conn.execute("CREATE TABLE users (id INTEGER PRIMARY KEY AUTOINCREMENT)").unwrap();
    let table_names = load_table_names(&conn, None).unwrap();
    assert!(table_names.contains(&String::from("users")));
}

#[test]
fn load_table_names_excludes_diesel_metadata_tables() {
    let conn = SqliteConnection::establish(":memory:").unwrap();
    conn.execute("CREATE TABLE __diesel_metadata (id INTEGER PRIMARY KEY AUTOINCREMENT)").unwrap();
    let table_names = load_table_names(&conn, None).unwrap();
    assert!(!table_names.contains(&String::from("__diesel_metadata")));
}

#[test]
fn load_table_names_excludes_sqlite_metadata_tables() {
    let conn = SqliteConnection::establish(":memory:").unwrap();
    conn.execute("CREATE TABLE __diesel_metadata (id INTEGER PRIMARY KEY AUTOINCREMENT)").unwrap();
    conn.execute("CREATE TABLE users (id INTEGER PRIMARY KEY AUTOINCREMENT)").unwrap();
    let table_names = load_table_names(&conn, None);
    assert_eq!(vec![String::from("users")], table_names.unwrap());
}

#[test]
fn load_table_names_excludes_views() {
    let conn = SqliteConnection::establish(":memory:").unwrap();
    conn.execute("CREATE TABLE users (id INTEGER PRIMARY KEY AUTOINCREMENT)").unwrap();
    conn.execute("CREATE VIEW answer AS SELECT 42").unwrap();
    let table_names = load_table_names(&conn, None);
    assert_eq!(vec![String::from("users")], table_names.unwrap());
}

#[test]
fn load_table_names_returns_error_when_given_schema_name() {
    let conn = SqliteConnection::establish(":memory:").unwrap();
    // We're testing the error message rather than using #[should_panic]
    // to ensure this is returning an error and not actually panicking.
    let table_names = load_table_names(&conn, Some("stuff"));
    match table_names {
        Ok(_) => panic!("Expected load_table_names to return an error"),
        Err(e) => assert!(e.description().starts_with("sqlite cannot infer \
            schema for databases")),
    }
}
