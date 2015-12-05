mod data_structures;

use diesel::*;
use syntax::ast;
use syntax::codemap::Span;
use syntax::ext::base::*;
use syntax::parse::token::{InternedString, str_to_ident};
use syntax::ptr::P;
use syntax::util::small_vector::SmallVector;

use self::data_structures::*;

pub fn expand_load_table<'cx>(
    cx: &'cx mut ExtCtxt,
    sp: Span,
    tts: &[ast::TokenTree]
) -> Box<MacResult+'cx> {
    let mut exprs = match get_exprs_from_tts(cx, sp, tts) {
        Some(ref exprs) if exprs.is_empty() => {
            cx.span_err(sp, "load_table_from_schema! takes 2 arguments");
            return DummyResult::any(sp);
        }
        None => return DummyResult::any(sp),
        Some(exprs) => exprs.into_iter()
    };

    match load_table_body(cx, sp, &mut exprs) {
        Ok(res) => res,
        Err(res) => res,
    }
}

pub fn load_table_body<T: Iterator<Item=P<ast::Expr>>>(
    cx: &mut ExtCtxt,
    sp: Span,
    exprs: &mut T,
) -> Result<Box<MacResult>, Box<MacResult>> {
    let database_url = try!(next_str_lit(cx, sp, exprs));
    let table_name = try!(next_str_lit(cx, sp, exprs));
    let connection = try!(establish_connection(cx, sp, &database_url));
    table_macro_call(cx, sp, &connection, &table_name)
        .map(|item| MacEager::items(SmallVector::one(item)))
}

pub fn expand_infer_schema<'cx>(
    cx: &'cx mut ExtCtxt,
    sp: Span,
    tts: &[ast::TokenTree]
) -> Box<MacResult+'cx> {
    let mut exprs = match get_exprs_from_tts(cx, sp, tts) {
        Some(exprs) => exprs.into_iter(),
        None => return DummyResult::any(sp),
    };

    match infer_schema_body(cx, sp, &mut exprs) {
        Ok(res) => res,
        Err(res) => res,
    }
}

pub fn infer_schema_body<T: Iterator<Item=P<ast::Expr>>>(
    cx: &mut ExtCtxt,
    sp: Span,
    exprs: &mut T,
) -> Result<Box<MacResult>, Box<MacResult>> {
    let database_url = try!(next_str_lit(cx, sp, exprs));
    let connection = try!(establish_connection(cx, sp, &database_url));
    let table_names = try!(load_table_names(cx, sp, &connection));
    let impls = table_names.into_iter()
        .map(|n| table_macro_call(cx, sp, &connection, &n))
        .collect();
    Ok(MacEager::items(SmallVector::many(try!(impls))))
}

fn establish_connection(
    cx: &mut ExtCtxt,
    sp: Span,
    database_url: &str,
) -> Result<Connection, Box<MacResult>> {
    Connection::establish(database_url).map_err(|_| {
        cx.span_err(sp, "failed to establish a database connection");
        DummyResult::any(sp)
    })
}

fn table_macro_call(
    cx: &mut ExtCtxt,
    sp: Span,
    connection: &Connection,
    table_name: &str,
) -> Result<P<ast::Item>, Box<MacResult>> {
    match get_table_data(&connection, table_name) {
        Err(NotFound) => {
            cx.span_err(sp, &format!("no table exists named {}", table_name));
            Err(DummyResult::any(sp))
        }
        Err(_) => {
            cx.span_err(sp, "error loading schema");
            Err(DummyResult::any(sp))
        }
        Ok(data) => {
            let tokens = data.iter().map(|a| column_def_tokens(cx, a))
                .collect::<Vec<_>>();
            let table_name = str_to_ident(table_name);
            let item = quote_item!(cx, table! {
                $table_name {
                    $tokens
                }
            }).unwrap();
            Ok(item)
        }
    }
}

fn next_str_lit<T: Iterator<Item=P<ast::Expr>>>(
    cx: &mut ExtCtxt,
    sp: Span,
    exprs: &mut T,
) -> Result<InternedString, Box<MacResult>> {
    match expr_to_string(cx, exprs.next().unwrap(), "expected string literal") {
        Some((s, _)) => Ok(s),
        None => Err(DummyResult::any(sp)),
    }
}

fn get_table_data(conn: &Connection, table_name: &str) -> QueryResult<Vec<PgAttr>> {
    use self::data_structures::pg_attribute::dsl::*;
    use self::data_structures::pg_type::dsl::{pg_type, typname};
    let t_oid = try!(table_oid(conn, table_name));

    pg_attribute.inner_join(pg_type)
        .select((attname, typname, attnotnull))
        .filter(attrelid.eq(t_oid))
        .filter(attnum.gt(0).and(attisdropped.ne(true)))
        .order(attnum)
        .load(&conn)
        .map(|r| r.collect::<Vec<_>>())
}

fn table_oid(conn: &Connection, table_name: &str) -> QueryResult<u32> {
    use self::data_structures::pg_class::dsl::*;
    pg_class.select(oid).filter(relname.eq(table_name)).first(&conn)
}

fn column_def_tokens(cx: &mut ExtCtxt, attr: &PgAttr) -> Vec<ast::TokenTree> {
    let column_name = str_to_ident(&attr.column_name);
    let type_name = str_to_ident(&capitalize(&attr.type_name));
    if attr.nullable {
        quote_tokens!(cx, $column_name -> Nullable<$type_name>,)
    } else {
        quote_tokens!(cx, $column_name -> $type_name,)
    }
}

fn capitalize(name: &str) -> String {
    name[..1].to_uppercase() + &name[1..]
}

fn load_table_names(
    cx: &mut ExtCtxt,
    sp: Span,
    connection: &Connection,
) -> Result<Vec<String>, Box<MacResult>> {
    connection.query_sql::<types::VarChar, String>(
        "SELECT table_name FROM information_schema.tables WHERE table_schema = 'public'")
        .map(|r| r.collect())
        .map_err(|_| {
            cx.span_err(sp, "Error loading table names");
            DummyResult::any(sp)
        })
}
