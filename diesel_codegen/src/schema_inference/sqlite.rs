use diesel::*;
use diesel::expression::dsl::sql;
use diesel::sqlite::{SqliteConnection, Sqlite};
use diesel::types::{HasSqlType, FromSqlRow};
use syntax::ast;
use syntax::codemap::Span;
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
    type_name.contains("char")||
        type_name.contains("clob")||
        type_name.contains("text")
}

pub fn determine_column_type(cx: &mut ExtCtxt, attr: &ColumnInformation) -> P<ast::Ty> {
    let type_name=attr.type_name.to_lowercase();
    let tpe = if type_name.contains("int") {
        quote_ty!(cx, ::diesel::types::BigInteger)
    } else if is_text(&type_name) {
        quote_ty!(cx, ::diesel::types::Text)
    } else if type_name.contains("blob") || type_name.is_empty() {
        quote_ty!(cx, ::diesel::types::Binary)
    } else {
        quote_ty!(cx, ::diesel::types::Double)
    };

    if attr.nullable {
        quote_ty!(cx, Nullable<$tpe>)
    } else {
        tpe
    }
}

pub fn load_table_names(
    _cx: &mut ExtCtxt,
    _sp: Span,
    connection: &SqliteConnection,
) -> Result<Vec<String>, result::Error> {
    use diesel::prelude::*;
    use diesel::expression::dsl::sql;

    let query = select(sql::<types::VarChar>("name FROM sqlite_master"))
        .filter(sql::<types::Bool>("type='table' AND name NOT LIKE '\\_\\_%'"));
    query.load(connection)
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
