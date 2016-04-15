use diesel::*;
use diesel::expression::dsl::sql;
use diesel::sqlite::{SqliteConnection, Sqlite};
use diesel::types::{HasSqlType, FromSqlRow};
use syntax::ast;
use syntax::ext::base::*;
use syntax::ptr::P;

use super::data_structures::*;

table!{
    pragma_table_info (cid){
        cid ->Integer,
        name -> VarChar,
        type_name -> VarChar,
        notnull -> Bool,
        dflt_value -> Nullable<VarChar>,
        pk -> Bool,
    }
}

pub fn get_table_data(conn: &SqliteConnection, table_name: &str)
    -> QueryResult<Vec<ColumnInformation>>
{
    let query = format!("PRAGMA TABLE_INFO('{}')", table_name);
    sql::<pragma_table_info::SqlType>(&query).load(conn)
}

fn is_text(type_name: &str) -> bool {
    type_name.contains("clob") ||
        type_name.contains("text")
}

pub fn determine_column_type(cx: &mut ExtCtxt, attr: &ColumnInformation) -> P<ast::Ty> {
    let type_name = attr.type_name.to_lowercase();
    let tpe = match &*type_name {
        "tinyint" => quote_ty!(cx, ::diesel::types::Bool),
        "smallint" | "int2" => quote_ty!(cx, ::diesel::types::SmallInt),
        "int" | "integer" | "int4" => quote_ty!(cx, ::diesel::types::Integer),
        "bigint" => quote_ty!(cx, ::diesel::types::BigInt),
        _ if type_name.contains("char") => quote_ty!(cx, ::diesel::types::VarChar),
        _ if is_text(&type_name) => quote_ty!(cx, ::diesel::types::Text),
        _ if type_name.contains("blob") || type_name.is_empty() => {
            quote_ty!(cx, ::diesel::types::Binary)
        }
        "float" => quote_ty!(cx, ::diesel::types::Float),
        "double" | "real" | "double precision" => quote_ty!(cx, ::diesel::types::Double),
        _ => {
            cx.span_err(cx.original_span(), &format!("Unsupported type: {}", type_name));
            quote_ty!(cx, ())
        }
    };

    if attr.nullable {
        quote_ty!(cx, Nullable<$tpe>)
    } else {
        tpe
    }
}

table! {
    sqlite_master (name) {
        name -> VarChar,
    }
}

pub fn load_table_names(connection: &SqliteConnection) -> QueryResult<Vec<String>> {
    use self::sqlite_master::dsl::*;

    sqlite_master.select(name)
        .filter(name.not_like("\\_\\_%").escape('\\'))
        .filter(name.not_like("sqlite%"))
        .filter(sql("type='table'"))
        .load(connection)
}

struct FullTableInfo {
    _cid: i32,
    name: String,
    _type_name: String,
    _not_null: bool,
    _dflt_value: Option<String>,
    primary_key: bool,
}

impl<ST> Queryable<ST, Sqlite> for FullTableInfo where
    Sqlite: HasSqlType<ST>,
    (i32, String, String, bool, Option<String>, bool): FromSqlRow<ST, Sqlite>,
{
    type Row = (i32, String, String, bool, Option<String>, bool);

    fn build(row: Self::Row) -> Self {
        FullTableInfo {
            _cid: row.0,
            name: row.1,
            _type_name: row.2,
            _not_null: row.3,
            _dflt_value: row.4,
            primary_key: row.5,
        }
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
