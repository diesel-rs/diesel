use diesel::*;
use diesel::pg::PgConnection;
use syntax::ast;
use syntax::codemap::Span;
use syntax::ext::base::*;
use syntax::ptr::P;
use syntax::parse::token::str_to_ident;

use super::data_structures::*;

table! {
    pg_attribute (attrelid) {
        attrelid -> Oid,
        attname -> VarChar,
        atttypid -> Oid,
        attnotnull -> Bool,
        attnum -> SmallInt,
        attisdropped -> Bool,
    }
}

table! {
    pg_type (oid) {
        oid -> Oid,
        typname -> VarChar,
    }
}

joinable!(pg_attribute -> pg_type (atttypid));
select_column_workaround!(pg_attribute -> pg_type (attrelid, attname, atttypid, attnotnull, attnum, attisdropped));
select_column_workaround!(pg_type -> pg_attribute (oid, typname));

table! {
    pg_index (indrelid) {
        indrelid -> Oid,
        indexrelid -> Oid,
        indkey -> Array<SmallInt>,
        indisprimary -> Bool,
    }
}

table! {
    pg_class (oid) {
        oid -> Oid,
        relname -> VarChar,
    }
}

pub fn determine_column_type(cx: &mut ExtCtxt, attr: &ColumnInformation) -> P<ast::Ty> {
    let tpe = if attr.type_name.starts_with("_") {
        let subtype = str_to_ident(&capitalize(&attr.type_name[1..]));
        quote_ty!(cx, Array<$subtype>)
    } else {
        let type_name = str_to_ident(&capitalize(&attr.type_name));
        quote_ty!(cx, $type_name)
    };

    if attr.nullable {
        quote_ty!(cx, Nullable<$tpe>)
    } else {
        tpe
    }
}

fn capitalize(name: &str) -> String {
    name[..1].to_uppercase() + &name[1..]
}

pub fn load_table_names(
    _cx: &mut ExtCtxt,
    _sp: Span,
    connection: &PgConnection,
) -> Result<Vec<String>, result::Error>
{
    use diesel::prelude::*;
    use diesel::expression::dsl::sql;

    let query = select(sql::<types::VarChar>("table_name FROM information_schema.tables"))
        .filter(sql::<types::Bool>("table_schema = 'public' AND table_name NOT LIKE '\\_\\_%'"));
    query.load(connection)
}

pub fn get_table_data(conn: &PgConnection, table_name: &str) -> QueryResult<Vec<ColumnInformation>> {
    use self::pg_attribute::dsl::*;
    use self::pg_type::dsl::{pg_type, typname};
    use self::pg_class::dsl::*;

    let table_oid = pg_class.select(oid).filter(relname.eq(table_name)).limit(1);

    pg_attribute.inner_join(pg_type)
        .select((attname, typname, attnotnull))
        .filter(attrelid.eq_any(table_oid))
        .filter(attnum.gt(0).and(attisdropped.ne(true)))
        .order(attnum)
        .load(conn)
}


pub fn get_primary_keys(conn: &PgConnection, table_name: &str) -> QueryResult<Vec<String>> {
    use self::pg_attribute::dsl::*;
    use self::pg_index::dsl::{pg_index, indisprimary, indexrelid, indrelid};
    use self::pg_class::dsl::*;

    let table_oid = pg_class.select(oid).filter(relname.eq(table_name)).limit(1);

    let pk_query = pg_index.select(indexrelid)
        .filter(indrelid.eq_any(table_oid))
        .filter(indisprimary.eq(true));

    pg_attribute.select(attname)
        .filter(attrelid.eq_any(pk_query))
        .order(attnum)
        .load(conn)
}
