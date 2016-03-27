mod data_structures;
#[cfg(feature = "postgres")]
mod pg;
#[cfg(feature = "sqlite")]
mod sqlite;

use diesel::{QueryResult, Connection};
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
    let table_names = load_table_names(cx, sp, &connection).unwrap();
    let impls = table_names.into_iter()
        .map(|n| table_macro_call(cx, sp, &connection, &n))
        .collect();
    Ok(MacEager::items(SmallVector::many(try!(impls))))
}

fn table_macro_call(
    cx: &mut ExtCtxt,
    sp: Span,
    connection: &InferConnection,
    table_name: &str,
) -> Result<P<ast::Item>, Box<MacResult>> {
    match get_table_data(connection, table_name) {
        Err(::diesel::result::Error::NotFound) => {
            cx.span_err(sp, &format!("no table exists named {}", table_name));
            Err(DummyResult::any(sp))
        }
        Err(_) => {
            cx.span_err(sp, "error loading schema");
            Err(DummyResult::any(sp))
        }
        Ok(data) => {
            let tokens = data.iter().map(|a| column_def_tokens(cx, a, &connection))
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

fn column_def_tokens(cx: &mut ExtCtxt, attr: &ColumnInformation, conn: &InferConnection)
    -> Vec<ast::TokenTree>
{
    let column_name = str_to_ident(&attr.column_name);
    let tpe = determine_column_type(cx, attr, conn);
    quote_tokens!(cx, $column_name -> $tpe,)
}

fn establish_real_connection<Conn>(
    cx: &mut ExtCtxt,
    sp: Span,
    database_url: &str,
) -> Result<Conn, Box<MacResult>> where
    Conn: Connection,
{
    Conn::establish(database_url).map_err(|error| {
        let error_message = format!(
            "Failed to establish a database connection at {}. Error: {:?}",
            database_url,
            error,
        );
        cx.span_err(sp, &error_message);
        DummyResult::any(sp)
    })
}

fn establish_connection(
    cx: &mut ExtCtxt,
    sp: Span,
    database_url: &str,
) -> Result<InferConnection, Box<MacResult>> {
    match database_url {
        #[cfg(feature = "postgres")]
        _ if database_url.starts_with("postgres://") || database_url.starts_with("postgresql://") => {
            establish_real_connection(cx, sp, database_url).map(|c| InferConnection::Pg(c))
        }
        #[cfg(feature = "sqlite")]
        _ => establish_real_connection(cx, sp, database_url).map(|c| InferConnection::Sqlite(c)),
        #[cfg(not(feature = "sqlite"))]
        _ => {
            let error_message = format!(
                "{} is not a valid PG database URL. \
                It must start with postgres:// or postgresql://",
                database_url,
            );
            cx.span_err(sp, &error_message);
            Err(DummyResult::any(sp))
        }
    }
}

fn get_table_data(conn: &InferConnection, table_name: &str)
    -> QueryResult<Vec<ColumnInformation>>
{
    match *conn {
        #[cfg(feature = "sqlite")]
        InferConnection::Sqlite(ref c) => sqlite::get_table_data(c, table_name),
        #[cfg(feature = "postgres")]
        InferConnection::Pg(ref c) => pg::get_table_data(c, table_name),
    }
}

fn load_table_names(
    cx: &mut ExtCtxt,
    sp: Span,
    connection: &InferConnection,
) -> Result<Vec<String>, ::diesel::result::Error> {
    match *connection {
        #[cfg(feature = "sqlite")]
        InferConnection::Sqlite(ref c) => sqlite::load_table_names(cx, sp, c),
        #[cfg(feature = "postgres")]
        InferConnection::Pg(ref c) => pg::load_table_names(cx, sp, c),
    }
}

fn determine_column_type(cx: &mut ExtCtxt, attr: &ColumnInformation, conn: &InferConnection)
    -> P<ast::Ty>
{
    match *conn {
        #[cfg(feature = "sqlite")]
        InferConnection::Sqlite(_) => sqlite::determine_column_type(cx, attr),
        #[cfg(feature = "postgres")]
        InferConnection::Pg(_) => pg::determine_column_type(cx, attr),
    }
}
