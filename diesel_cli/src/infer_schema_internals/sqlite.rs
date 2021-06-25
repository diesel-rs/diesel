use std::error::Error;

use diesel::dsl::sql;
use diesel::*;

use super::data_structures::*;
use super::table_data::TableName;
use crate::print_schema::ColumnSorting;

table! {
    sqlite_master (name) {
        name -> VarChar,
    }
}

table! {
    pragma_table_info (cid) {
        cid ->Integer,
        name -> VarChar,
        type_name -> VarChar,
        notnull -> Bool,
        dflt_value -> Nullable<VarChar>,
        pk -> Bool,
    }
}

table! {
    pragma_foreign_key_list {
        id -> Integer,
        seq -> Integer,
        _table -> VarChar,
        from -> VarChar,
        to -> VarChar,
        on_update -> VarChar,
        on_delete -> VarChar,
        _match -> VarChar,
    }
}

pub fn load_table_names(
    connection: &mut SqliteConnection,
    schema_name: Option<&str>,
) -> Result<Vec<TableName>, Box<dyn Error + Send + Sync + 'static>> {
    use self::sqlite_master::dsl::*;

    if schema_name.is_some() {
        return Err("sqlite cannot infer schema for databases other than the \
                    main database"
            .into());
    }

    Ok(sqlite_master
        .select(name)
        .filter(name.not_like("\\_\\_%").escape('\\'))
        .filter(name.not_like("sqlite%"))
        .filter(sql::<sql_types::Bool>("type='table'"))
        .order(name)
        .load::<String>(connection)?
        .into_iter()
        .map(TableName::from_name)
        .collect())
}

pub fn load_foreign_key_constraints(
    connection: &mut SqliteConnection,
    schema_name: Option<&str>,
) -> Result<Vec<ForeignKeyConstraint>, Box<dyn Error + Send + Sync + 'static>> {
    let tables = load_table_names(connection, schema_name)?;
    let rows = tables
        .into_iter()
        .map(|child_table| {
            let query = format!("PRAGMA FOREIGN_KEY_LIST('{}')", child_table.sql_name);
            Ok(sql::<pragma_foreign_key_list::SqlType>(&query)
                .load::<ForeignKeyListRow>(connection)?
                .into_iter()
                .map(|row| {
                    let parent_table = TableName::from_name(row.parent_table);
                    ForeignKeyConstraint {
                        child_table: child_table.clone(),
                        parent_table,
                        foreign_key: row.foreign_key.clone(),
                        foreign_key_rust_name: row.foreign_key,
                        primary_key: row.primary_key,
                    }
                })
                .collect())
        })
        .collect::<QueryResult<Vec<Vec<_>>>>()?;
    Ok(rows.into_iter().flatten().collect())
}

pub fn get_table_data(
    conn: &mut SqliteConnection,
    table: &TableName,
    column_sorting: &ColumnSorting,
) -> QueryResult<Vec<ColumnInformation>> {
    let query = format!("PRAGMA TABLE_INFO('{}')", &table.sql_name);
    let mut result = sql::<pragma_table_info::SqlType>(&query).load(conn)?;
    match column_sorting {
        ColumnSorting::OrdinalPosition => {}
        ColumnSorting::Name => {
            result.sort_by(|a: &ColumnInformation, b: &ColumnInformation| {
                a.column_name.partial_cmp(&b.column_name).unwrap()
            });
        }
    };
    Ok(result)
}

#[derive(Queryable)]
struct FullTableInfo {
    _cid: i32,
    name: String,
    _type_name: String,
    _not_null: bool,
    _dflt_value: Option<String>,
    primary_key: bool,
}

#[derive(Queryable)]
struct ForeignKeyListRow {
    _id: i32,
    _seq: i32,
    parent_table: String,
    foreign_key: String,
    primary_key: String,
    _on_update: String,
    _on_delete: String,
    _match: String,
}

pub fn get_primary_keys(
    conn: &mut SqliteConnection,
    table: &TableName,
) -> QueryResult<Vec<String>> {
    let query = format!("PRAGMA TABLE_INFO('{}')", &table.sql_name);
    let results = sql::<pragma_table_info::SqlType>(&query).load::<FullTableInfo>(conn)?;
    Ok(results
        .into_iter()
        .filter_map(|i| if i.primary_key { Some(i.name) } else { None })
        .collect())
}

pub fn determine_column_type(
    attr: &ColumnInformation,
) -> Result<ColumnType, Box<dyn Error + Send + Sync + 'static>> {
    let type_name = attr.type_name.to_lowercase();
    let path = if is_bool(&type_name) {
        String::from("Bool")
    } else if is_smallint(&type_name) {
        String::from("SmallInt")
    } else if is_bigint(&type_name) {
        String::from("BigInt")
    } else if type_name.contains("int") {
        String::from("Integer")
    } else if is_text(&type_name) {
        String::from("Text")
    } else if is_binary(&type_name) {
        String::from("Binary")
    } else if is_float(&type_name) {
        String::from("Float")
    } else if is_double(&type_name) {
        String::from("Double")
    } else if type_name == "datetime" || type_name == "timestamp" {
        String::from("Timestamp")
    } else if type_name == "date" {
        String::from("Date")
    } else if type_name == "time" {
        String::from("Time")
    } else {
        return Err(format!("Unsupported type: {}", type_name).into());
    };

    Ok(ColumnType {
        schema: None,
        rust_name: path.clone(),
        sql_name: path,
        is_array: false,
        is_nullable: attr.nullable,
        is_unsigned: false,
    })
}

fn is_text(type_name: &str) -> bool {
    type_name.contains("char") || type_name.contains("clob") || type_name.contains("text")
}

fn is_binary(type_name: &str) -> bool {
    type_name.contains("blob") || type_name.contains("binary") || type_name.is_empty()
}

fn is_bool(type_name: &str) -> bool {
    type_name == "boolean" || type_name.contains("tiny") && type_name.contains("int")
}

fn is_smallint(type_name: &str) -> bool {
    type_name == "int2" || type_name.contains("small") && type_name.contains("int")
}

fn is_bigint(type_name: &str) -> bool {
    type_name == "int8" || type_name.contains("big") && type_name.contains("int")
}

fn is_float(type_name: &str) -> bool {
    type_name.contains("float") || type_name.contains("real")
}

fn is_double(type_name: &str) -> bool {
    type_name.contains("double") || type_name.contains("num") || type_name.contains("dec")
}

#[test]
fn load_table_names_returns_nothing_when_no_tables_exist() {
    let mut conn = SqliteConnection::establish(":memory:").unwrap();
    assert_eq!(
        Vec::<TableName>::new(),
        load_table_names(&mut conn, None).unwrap()
    );
}

#[test]
fn load_table_names_includes_tables_that_exist() {
    let mut conn = SqliteConnection::establish(":memory:").unwrap();
    conn.execute("CREATE TABLE users (id INTEGER PRIMARY KEY AUTOINCREMENT)")
        .unwrap();
    let table_names = load_table_names(&mut conn, None).unwrap();
    assert!(table_names.contains(&TableName::from_name("users")));
}

#[test]
fn load_table_names_excludes_diesel_metadata_tables() {
    let mut conn = SqliteConnection::establish(":memory:").unwrap();
    conn.execute("CREATE TABLE __diesel_metadata (id INTEGER PRIMARY KEY AUTOINCREMENT)")
        .unwrap();
    let table_names = load_table_names(&mut conn, None).unwrap();
    assert!(!table_names.contains(&TableName::from_name("__diesel_metadata")));
}

#[test]
fn load_table_names_excludes_sqlite_metadata_tables() {
    let mut conn = SqliteConnection::establish(":memory:").unwrap();
    conn.execute("CREATE TABLE __diesel_metadata (id INTEGER PRIMARY KEY AUTOINCREMENT)")
        .unwrap();
    conn.execute("CREATE TABLE users (id INTEGER PRIMARY KEY AUTOINCREMENT)")
        .unwrap();
    let table_names = load_table_names(&mut conn, None);
    assert_eq!(vec![TableName::from_name("users")], table_names.unwrap());
}

#[test]
fn load_table_names_excludes_views() {
    let mut conn = SqliteConnection::establish(":memory:").unwrap();
    conn.execute("CREATE TABLE users (id INTEGER PRIMARY KEY AUTOINCREMENT)")
        .unwrap();
    conn.execute("CREATE VIEW answer AS SELECT 42").unwrap();
    let table_names = load_table_names(&mut conn, None);
    assert_eq!(vec![TableName::from_name("users")], table_names.unwrap());
}

#[test]
fn load_table_names_returns_error_when_given_schema_name() {
    let mut conn = SqliteConnection::establish(":memory:").unwrap();
    // We're testing the error message rather than using #[should_panic]
    // to ensure this is returning an error and not actually panicking.
    let table_names = load_table_names(&mut conn, Some("stuff"));
    match table_names {
        Ok(_) => panic!("Expected load_table_names to return an error"),
        Err(e) => assert!(e.to_string().starts_with(
            "sqlite cannot infer \
             schema for databases"
        )),
    }
}

#[test]
fn load_table_names_output_is_ordered() {
    let mut conn = SqliteConnection::establish(":memory:").unwrap();
    conn.execute("CREATE TABLE bbb (id INTEGER PRIMARY KEY AUTOINCREMENT)")
        .unwrap();
    conn.execute("CREATE TABLE aaa (id INTEGER PRIMARY KEY AUTOINCREMENT)")
        .unwrap();
    conn.execute("CREATE TABLE ccc (id INTEGER PRIMARY KEY AUTOINCREMENT)")
        .unwrap();

    let table_names = load_table_names(&mut conn, None)
        .unwrap()
        .iter()
        .map(|table| table.to_string())
        .collect::<Vec<_>>();
    assert_eq!(vec!["aaa", "bbb", "ccc"], table_names);
}

#[test]
fn load_foreign_key_constraints_loads_foreign_keys() {
    let mut connection = SqliteConnection::establish(":memory:").unwrap();

    connection.execute("CREATE TABLE table_1 (id)").unwrap();
    connection
        .execute("CREATE TABLE table_2 (id, fk_one REFERENCES table_1(id))")
        .unwrap();
    connection
        .execute("CREATE TABLE table_3 (id, fk_two REFERENCES table_2(id))")
        .unwrap();

    let table_1 = TableName::from_name("table_1");
    let table_2 = TableName::from_name("table_2");
    let table_3 = TableName::from_name("table_3");
    let fk_one = ForeignKeyConstraint {
        child_table: table_2.clone(),
        parent_table: table_1.clone(),
        foreign_key: "fk_one".into(),
        foreign_key_rust_name: "fk_one".into(),
        primary_key: "id".into(),
    };
    let fk_two = ForeignKeyConstraint {
        child_table: table_3.clone(),
        parent_table: table_2.clone(),
        foreign_key: "fk_two".into(),
        foreign_key_rust_name: "fk_two".into(),
        primary_key: "id".into(),
    };
    let fks = load_foreign_key_constraints(&mut connection, None).unwrap();
    assert_eq!(vec![fk_one, fk_two], fks);
}
