use diesel_codegen_shared::*;
use syntax::ast;
use syntax::codemap::Span;
use syntax::ext::base::*;
use syntax::ext::build::AstBuilder;
use syntax::parse::token::{self, InternedString, str_to_ident};
use syntax::ptr::P;
use syntax::util::small_vector::SmallVector;
use syntax::tokenstream::TokenTree;

use util::comma_delimited_tokens;

pub fn expand_load_table<'cx>(
    cx: &'cx mut ExtCtxt,
    sp: Span,
    tts: &[TokenTree]
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
    let database_url = try!(database_url(cx, sp, exprs));
    let table_name = try!(next_str_lit(cx, sp, exprs));
    let connection = match establish_connection(&database_url) {
        Ok(conn) => conn,
        Err(e) => {
            cx.span_err(sp, &e.to_string());
            return Err(DummyResult::any(sp));
        }
    };
    table_macro_call(cx, sp, &connection, &table_name)
        .map(|item| MacEager::items(SmallVector::one(item)))
}

pub fn expand_infer_schema<'cx>(
    cx: &'cx mut ExtCtxt,
    sp: Span,
    tts: &[TokenTree]
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
    let table_names = load_table_names(&database_url).unwrap();
    let impls = table_names.into_iter()
        .map(|n| quote_item!(cx, infer_table_from_schema!($database_url, $n);).unwrap())
        .collect();
    Ok(MacEager::items(SmallVector::many(impls)))
}

fn table_macro_call(
    cx: &mut ExtCtxt,
    sp: Span,
    connection: &InferConnection,
    table_name: &str,
) -> Result<P<ast::Item>, Box<MacResult>> {
    match get_table_data(connection, table_name) {
        Err(e) => {
            cx.span_err(sp, &e.to_string());
            Err(DummyResult::any(sp))
        }
        Ok(data) => {
            let primary_keys = match get_primary_keys(connection, table_name) {
                Ok(keys) => keys,
                Err(e) => {
                    cx.span_err(sp, &e.to_string());
                    return Err(DummyResult::any(sp));
                }
            };
            let tokens = data.iter().map(|a| column_def_tokens(cx, sp, a, &connection))
                .collect::<Vec<_>>();
            let table_name = str_to_ident(table_name);
            let primary_key_tokens = primary_keys.iter()
                .map(|s| str_to_ident(&s))
                .map(token::Ident);
            let primary_key = comma_delimited_tokens(primary_key_tokens, sp);
            let item = quote_item!(cx, table! {
                $table_name ($primary_key) {
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

fn column_def_tokens(cx: &mut ExtCtxt, span: Span, attr: &ColumnInformation, conn: &InferConnection)
    -> Vec<TokenTree>
{
    let column_name = str_to_ident(&attr.column_name);
    let tpe = match determine_column_type(attr, conn) {
        Ok(ty) => {
            let idents = ty.path.iter().map(|a| str_to_ident(&a)).collect();
            let path = cx.path_global(span, idents);
            let mut path = quote_ty!(cx, $path);
            if ty.is_array {
                path = quote_ty!(cx, Array<$path>);
            }
            if ty.is_nullable {
                path = quote_ty!(cx, Nullable<$path>);
            }
            path
        }
        Err(e) => {
            cx.span_err(span, &e.to_string());
            quote_ty!(cx, ())
        }
    };
    quote_tokens!(cx, $column_name -> $tpe,)
}

fn database_url<T: Iterator<Item=P<ast::Expr>>>(
    cx: &mut ExtCtxt,
    sp: Span,
    exprs: &mut T,
) -> Result<String, Box<MacResult>> {
    let database_url = try!(next_str_lit(cx, sp, exprs));
    match extract_database_url(&database_url) {
        Ok(s) => Ok(s.into_owned()),
        Err(msg) => {
            cx.span_err(sp, &msg);
            Err(DummyResult::any(sp))
        }
    }
}
