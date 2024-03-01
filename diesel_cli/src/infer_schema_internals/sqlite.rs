use std::fmt;

use diesel::deserialize::Queryable;
use diesel::dsl::sql;
use diesel::row::NamedRow;
use diesel::sql_types::{Bool, Text};
use diesel::sqlite::Sqlite;
use diesel::*;

use super::data_structures::*;
use super::table_data::TableName;
use crate::config::PrintSchema;
use crate::print_schema::ColumnSorting;

table! {
    sqlite_master (name) {
        name -> VarChar,
    }
}

table! {
    pragma_foreign_key_list {
        id -> Integer,
        seq -> Integer,
        _table -> VarChar,
        from -> VarChar,
        to -> Nullable<VarChar>,
        on_update -> VarChar,
        on_delete -> VarChar,
        _match -> VarChar,
    }
}

pub fn load_table_names(
    connection: &mut SqliteConnection,
    schema_name: Option<&str>,
) -> Result<Vec<TableName>, crate::errors::Error> {
    use self::sqlite_master::dsl::*;

    if schema_name.is_some() {
        return Err(crate::errors::Error::InvalidSqliteSchema);
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
) -> Result<Vec<ForeignKeyConstraint>, crate::errors::Error> {
    let tables = load_table_names(connection, schema_name)?;
    let rows = tables
        .into_iter()
        .map(|child_table| {
            let query = format!("PRAGMA FOREIGN_KEY_LIST('{}')", child_table.sql_name);
            sql::<pragma_foreign_key_list::SqlType>(&query)
                .load::<ForeignKeyListRow>(connection)?
                .into_iter()
                .map(|row| {
                    let parent_table = TableName::from_name(row.parent_table);
                    let primary_key = if let Some(primary_key) = row.primary_key {
                        vec![primary_key]
                    } else {
                        get_primary_keys(connection, &parent_table)?
                    };
                    Ok(ForeignKeyConstraint {
                        child_table: child_table.clone(),
                        parent_table,
                        foreign_key_columns: vec![row.foreign_key.clone()],
                        foreign_key_columns_rust: vec![row.foreign_key.clone()],
                        primary_key_columns: primary_key,
                    })
                })
                .collect::<Result<_, _>>()
        })
        .collect::<QueryResult<Vec<Vec<_>>>>()?;
    Ok(rows.into_iter().flatten().collect())
}

#[derive(PartialEq, Eq, PartialOrd, Ord)]
struct SqliteVersion {
    major: u32,
    minor: u32,
    patch: u32,
}

impl SqliteVersion {
    pub fn new(major: u32, minor: u32, patch: u32) -> SqliteVersion {
        SqliteVersion {
            major,
            minor,
            patch,
        }
    }
}

impl fmt::Display for SqliteVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}

fn get_sqlite_version(conn: &mut SqliteConnection) -> QueryResult<SqliteVersion> {
    let query = "SELECT sqlite_version()";
    let result = sql::<sql_types::Text>(query).load::<String>(conn)?;
    let parts = result[0]
        .split('.')
        .map(|part| {
            part.parse()
                .expect("sqlite version is guaranteed to consist of numbers")
        })
        .collect::<Vec<u32>>();
    assert_eq!(parts.len(), 3);
    Ok(SqliteVersion::new(parts[0], parts[1], parts[2]))
}

// In sqlite the rowid is a signed 64-bit integer.
// See: https://sqlite.org/rowidtable.html
// We should use BigInt here but to avoid type problems with foreign keys to
// rowid columns this is for now not done. A patch can be used after the schema
// is generated to convert the columns to BigInt as needed.
const ROWID_TYPE_NAME: &str = "Integer";

pub fn get_table_data(
    conn: &mut SqliteConnection,
    table: &TableName,
    column_sorting: &ColumnSorting,
) -> QueryResult<Vec<ColumnInformation>> {
    let sqlite_version = get_sqlite_version(conn)?;
    let query = if sqlite_version >= SqliteVersion::new(3, 26, 0) {
        /*
         * To get generated columns we need to use TABLE_XINFO
         * This would return hidden columns as well, but those would need to be created at runtime
         * therefore they aren't an issue.
         */
        format!("PRAGMA TABLE_XINFO('{}')", &table.sql_name)
    } else {
        format!("PRAGMA TABLE_INFO('{}')", &table.sql_name)
    };

    // See: https://github.com/diesel-rs/diesel/issues/3579 as to why we use a direct
    // `sql_query` with `QueryableByName` instead of using `sql::<pragma_table_info::SqlType>`.
    let mut result = sql_query(query).load::<ColumnInformation>(conn)?;
    // Add implicit rowid primary key column if the only primary key is rowid
    // and ensure that the rowid column uses the right type.
    let primary_key = get_primary_keys(conn, table)?;
    if primary_key.len() == 1 {
        let primary_key = primary_key.first().expect("guaranteed to have one element");
        if !result.iter_mut().any(|x| &x.column_name == primary_key) {
            // Prepend implicit rowid column for the rowid implicit primary key.
            result.insert(
                0,
                ColumnInformation {
                    column_name: String::from(primary_key),
                    type_name: String::from(ROWID_TYPE_NAME),
                    type_schema: None,
                    nullable: false,
                    max_length: None,
                    comment: None,
                },
            );
        }
    }

    match column_sorting {
        ColumnSorting::OrdinalPosition => {}
        ColumnSorting::Name => {
            result.sort_by(|a: &ColumnInformation, b: &ColumnInformation| {
                a.column_name.cmp(&b.column_name)
            });
        }
    };
    Ok(result)
}

impl QueryableByName<Sqlite> for ColumnInformation {
    fn build<'a>(row: &impl NamedRow<'a, Sqlite>) -> deserialize::Result<Self> {
        let column_name = NamedRow::get::<Text, String>(row, "name")?;
        let type_name = NamedRow::get::<Text, String>(row, "type")?;
        let notnull = NamedRow::get::<Bool, bool>(row, "notnull")?;

        Ok(Self::new(
            column_name,
            type_name,
            None,
            !notnull,
            None,
            None,
        ))
    }
}

struct PrimaryKeyInformation {
    name: String,
    primary_key: bool,
}

impl QueryableByName<Sqlite> for PrimaryKeyInformation {
    fn build<'a>(row: &impl NamedRow<'a, Sqlite>) -> deserialize::Result<Self> {
        let name = NamedRow::get::<Text, String>(row, "name")?;
        let primary_key = NamedRow::get::<Bool, bool>(row, "pk")?;

        Ok(Self { name, primary_key })
    }
}

struct WithoutRowIdInformation {
    name: String,
    without_row_id: bool,
}

impl QueryableByName<Sqlite> for WithoutRowIdInformation {
    fn build<'a>(row: &impl NamedRow<'a, Sqlite>) -> deserialize::Result<Self> {
        Ok(Self {
            name: NamedRow::get::<Text, String>(row, "name")?,
            without_row_id: NamedRow::get::<Bool, bool>(row, "wr")?,
        })
    }
}

pub fn column_is_row_id(
    conn: &mut SqliteConnection,
    table: &TableName,
    primary_keys: &[String],
    column_name: &str,
    type_name: &str,
) -> Result<bool, crate::errors::Error> {
    let sqlite_version = get_sqlite_version(conn)?;
    if sqlite_version < SqliteVersion::new(3, 37, 0) {
        return Err(crate::errors::Error::UnsupportedFeature(format!(
            "Parameter `sqlite_integer_primary_key_is_bigint` needs SQLite 3.37 or above. \
            Your current SQLite version is {sqlite_version}."
        )));
    }

    if type_name != "integer" {
        return Ok(false);
    }

    if !matches!(primary_keys, [pk] if pk == column_name) {
        return Ok(false);
    }

    let table_list_query = format!("PRAGMA TABLE_LIST('{}')", &table.sql_name);
    let table_list_results = sql_query(table_list_query).load::<WithoutRowIdInformation>(conn)?;

    let res = table_list_results
        .iter()
        .find(|wr_info| wr_info.name == table.sql_name)
        .map(|wr_info| !wr_info.without_row_id)
        .unwrap_or_default();

    Ok(res)
}

#[derive(Queryable)]
struct ForeignKeyListRow {
    _id: i32,
    _seq: i32,
    parent_table: String,
    foreign_key: String,
    primary_key: Option<String>,
    _on_update: String,
    _on_delete: String,
    _match: String,
}

/// All SQLite rowid aliases
/// Ordered by preference
/// https://www.sqlite.org/rowidtable.html
const SQLITE_ROWID_ALIASES: &[&str] = &["rowid", "oid", "_rowid_"];

pub fn get_primary_keys(
    conn: &mut SqliteConnection,
    table: &TableName,
) -> QueryResult<Vec<String>> {
    let sqlite_version = get_sqlite_version(conn)?;
    let query = if sqlite_version >= SqliteVersion::new(3, 26, 0) {
        format!("PRAGMA TABLE_XINFO('{}')", &table.sql_name)
    } else {
        format!("PRAGMA TABLE_INFO('{}')", &table.sql_name)
    };
    let results = sql_query(query).load::<PrimaryKeyInformation>(conn)?;
    let mut collected: Vec<String> = results
        .iter()
        .filter_map(|i| {
            if i.primary_key {
                Some(i.name.clone())
            } else {
                None
            }
        })
        .collect();
    // SQLite tables without "WITHOUT ROWID" always have aliases for the implicit PRIMARY KEY "rowid" and its aliases
    // unless the user defines a column with those names, then the name in question refers to the created column
    // https://www.sqlite.org/rowidtable.html
    if collected.is_empty() {
        for alias in SQLITE_ROWID_ALIASES {
            if results.iter().any(|v| &v.name.as_str() == alias) {
                continue;
            }

            // only add one alias as the primary key
            collected.push(alias.to_string());
            break;
        }
        // if it is still empty at this point, then a "diesel requires a primary key" error will be given
    }
    Ok(collected)
}

#[tracing::instrument(skip(conn))]
pub fn determine_column_type(
    conn: &mut SqliteConnection,
    attr: &ColumnInformation,
    table: &TableName,
    primary_keys: &[String],
    config: &PrintSchema,
) -> Result<ColumnType, crate::errors::Error> {
    let mut type_name = attr.type_name.to_lowercase();
    if type_name == "generated always" {
        type_name.clear();
    }

    let path = if is_bool(&type_name) {
        String::from("Bool")
    } else if is_smallint(&type_name) {
        String::from("SmallInt")
    } else if is_bigint(&type_name) {
        String::from("BigInt")
    } else if type_name.contains("int") {
        let sqlite_integer_primary_key_is_bigint = config
            .sqlite_integer_primary_key_is_bigint
            .unwrap_or_default();

        if sqlite_integer_primary_key_is_bigint
            && column_is_row_id(conn, table, primary_keys, &attr.column_name, &type_name)?
        {
            String::from("BigInt")
        } else {
            String::from("Integer")
        }
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
        return Err(crate::errors::Error::UnsupportedType(type_name));
    };

    Ok(ColumnType {
        schema: None,
        rust_name: path.clone(),
        sql_name: path,
        is_array: false,
        is_nullable: attr.nullable,
        is_unsigned: false,
        max_length: attr.max_length,
    })
}

fn is_text(type_name: &str) -> bool {
    type_name.contains("char") || type_name.contains("clob") || type_name.contains("text")
}

fn is_binary(type_name: &str) -> bool {
    type_name.contains("blob") || type_name.contains("binary") || type_name.is_empty()
}

fn is_bool(type_name: &str) -> bool {
    type_name == "boolean"
        || type_name == "bool"
        || type_name.contains("tiny") && type_name.contains("int")
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
    diesel::sql_query("CREATE TABLE users (id INTEGER PRIMARY KEY AUTOINCREMENT)")
        .execute(&mut conn)
        .unwrap();
    let table_names = load_table_names(&mut conn, None).unwrap();
    assert!(table_names.contains(&TableName::from_name("users")));
}

#[test]
fn load_table_names_excludes_diesel_metadata_tables() {
    let mut conn = SqliteConnection::establish(":memory:").unwrap();
    diesel::sql_query("CREATE TABLE __diesel_metadata (id INTEGER PRIMARY KEY AUTOINCREMENT)")
        .execute(&mut conn)
        .unwrap();
    let table_names = load_table_names(&mut conn, None).unwrap();
    assert!(!table_names.contains(&TableName::from_name("__diesel_metadata")));
}

#[test]
fn load_table_names_excludes_sqlite_metadata_tables() {
    let mut conn = SqliteConnection::establish(":memory:").unwrap();
    diesel::sql_query("CREATE TABLE __diesel_metadata (id INTEGER PRIMARY KEY AUTOINCREMENT)")
        .execute(&mut conn)
        .unwrap();
    diesel::sql_query("CREATE TABLE users (id INTEGER PRIMARY KEY AUTOINCREMENT)")
        .execute(&mut conn)
        .unwrap();
    let table_names = load_table_names(&mut conn, None);
    assert_eq!(vec![TableName::from_name("users")], table_names.unwrap());
}

#[test]
fn load_table_names_excludes_views() {
    let mut conn = SqliteConnection::establish(":memory:").unwrap();
    diesel::sql_query("CREATE TABLE users (id INTEGER PRIMARY KEY AUTOINCREMENT)")
        .execute(&mut conn)
        .unwrap();
    diesel::sql_query("CREATE VIEW answer AS SELECT 42")
        .execute(&mut conn)
        .unwrap();
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
    diesel::sql_query("CREATE TABLE bbb (id INTEGER PRIMARY KEY AUTOINCREMENT)")
        .execute(&mut conn)
        .unwrap();
    diesel::sql_query("CREATE TABLE aaa (id INTEGER PRIMARY KEY AUTOINCREMENT)")
        .execute(&mut conn)
        .unwrap();
    diesel::sql_query("CREATE TABLE ccc (id INTEGER PRIMARY KEY AUTOINCREMENT)")
        .execute(&mut conn)
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

    diesel::sql_query("CREATE TABLE table_1 (id)")
        .execute(&mut connection)
        .unwrap();
    diesel::sql_query("CREATE TABLE table_2 (id, fk_one REFERENCES table_1(id))")
        .execute(&mut connection)
        .unwrap();
    diesel::sql_query("CREATE TABLE table_3 (id, fk_two REFERENCES table_2(id))")
        .execute(&mut connection)
        .unwrap();

    let table_1 = TableName::from_name("table_1");
    let table_2 = TableName::from_name("table_2");
    let table_3 = TableName::from_name("table_3");
    let fk_one = ForeignKeyConstraint {
        child_table: table_2.clone(),
        parent_table: table_1,
        foreign_key_columns: vec!["fk_one".into()],
        foreign_key_columns_rust: vec!["fk_one".into()],
        primary_key_columns: vec!["id".into()],
    };
    let fk_two = ForeignKeyConstraint {
        child_table: table_3,
        parent_table: table_2,
        foreign_key_columns: vec!["fk_two".into()],
        foreign_key_columns_rust: vec!["fk_two".into()],
        primary_key_columns: vec!["id".into()],
    };

    let fks = load_foreign_key_constraints(&mut connection, None).unwrap();
    assert_eq!(vec![fk_one, fk_two], fks);
}

#[test]
fn all_rowid_aliases_used_empty_result() {
    let mut connection = SqliteConnection::establish(":memory:").unwrap();

    diesel::sql_query("CREATE TABLE table_1 (rowid TEXT, oid TEXT, _rowid_ TEXT)")
        .execute(&mut connection)
        .unwrap();

    let table_1 = TableName::from_name("table_1");

    let res = get_primary_keys(&mut connection, &table_1);
    assert!(res.is_ok());
    assert!(res.unwrap().is_empty());
}

#[test]
fn integer_primary_key_sqlite_3_37() {
    let mut conn = SqliteConnection::establish(":memory:").unwrap();

    let sqlite_version = get_sqlite_version(&mut conn).unwrap();
    if sqlite_version < SqliteVersion::new(3, 37, 0) {
        return;
    }

    let test_data = [
        (
            "table_1",
            "CREATE TABLE table_1 (id INTEGER PRIMARY KEY)",
            vec![("id", "Integer".to_owned())],
            vec![("id", "BigInt".to_owned())],
        ),
        (
            "table_2",
            "CREATE TABLE table_2 (id INTEGER, PRIMARY KEY(id))",
            vec![("id", "Integer".to_owned())],
            vec![("id", "BigInt".to_owned())],
        ),
        (
            "table_3",
            "CREATE TABLE table_3 (id INTEGER)",
            vec![
                ("rowid", "Integer".to_owned()),
                ("id", "Integer".to_owned()),
            ],
            vec![("rowid", "BigInt".to_owned()), ("id", "Integer".to_owned())],
        ),
        (
            "table_4",
            "CREATE TABLE table_4 (id1 INTEGER, id2 INTEGER, PRIMARY KEY(id1, id2))",
            vec![("id1", "Integer".to_owned()), ("id2", "Integer".to_owned())],
            vec![("id1", "Integer".to_owned()), ("id2", "Integer".to_owned())],
        ),
        (
            "table_5",
            "CREATE TABLE table_5 (id INT PRIMARY KEY)",
            vec![("id", "Integer".to_owned())],
            vec![("id", "Integer".to_owned())],
        ),
        (
            "table_6",
            "CREATE TABLE table_6 (id INTEGER PRIMARY KEY) WITHOUT ROWID",
            vec![("id", "Integer".to_owned())],
            vec![("id", "Integer".to_owned())],
        ),
    ];

    for (table_name, sql_query, expected_off_types, expected_on_types) in test_data {
        diesel::sql_query(sql_query).execute(&mut conn).unwrap();

        let table = TableName::from_name(table_name);
        let column_infos = get_table_data(&mut conn, &table, &Default::default()).unwrap();

        let primary_keys = get_primary_keys(&mut conn, &table).unwrap();

        let off_column_types = column_infos
            .iter()
            .map(|column_info| {
                (
                    column_info.column_name.as_str(),
                    determine_column_type(
                        &mut conn,
                        column_info,
                        &table,
                        &primary_keys,
                        &Default::default(),
                    )
                    .unwrap()
                    .sql_name,
                )
            })
            .collect::<Vec<_>>();

        let on_column_types = column_infos
            .iter()
            .map(|column_info| {
                (
                    column_info.column_name.as_str(),
                    determine_column_type(
                        &mut conn,
                        column_info,
                        &table,
                        &primary_keys,
                        &PrintSchema {
                            sqlite_integer_primary_key_is_bigint: Some(true),
                            ..Default::default()
                        },
                    )
                    .unwrap()
                    .sql_name,
                )
            })
            .collect::<Vec<_>>();

        assert_eq!(
            (table_name, off_column_types),
            (table_name, expected_off_types)
        );

        assert_eq!(
            (table_name, on_column_types),
            (table_name, expected_on_types)
        );
    }
}
