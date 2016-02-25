use diesel::*;
use diesel::sqlite::SqliteConnection;
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
        pk -> Integer,
    }
}



pub fn get_table_data(conn: &SqliteConnection, table_name: &str) -> QueryResult<Vec<ColumnInformation>>
{
    conn.execute_pragma::<pragma_table_info::SqlType, ColumnInformation>(
        &format!("PRAGMA TABLE_INFO('{}')", table_name))
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
) -> Result<Vec<String>, result::Error>
{
    use diesel::prelude::*;
    use diesel::expression::dsl::sql;

    let query = select(sql::<types::VarChar>("name FROM sqlite_master"))
        .filter(sql::<types::Bool>("type='table' AND name NOT LIKE '\\_\\_%'"));
    query.load(connection)
}
