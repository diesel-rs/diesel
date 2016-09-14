use syntax::ast::{self, MetaItem, NestedMetaItem, MetaItemKind, TyKind};
use syntax::codemap::Span;
use syntax::ext::base::{Annotatable, ExtCtxt};
use syntax::ptr::P;
use syntax::parse::token::{InternedString, str_to_ident};
use syntax::tokenstream::TokenTree;

use model::Model;
use util::lifetime_list_tokens;

pub fn expand_changeset_for(
    cx: &mut ExtCtxt,
    span: Span,
    meta_item: &MetaItem,
    annotatable: &Annotatable,
    push: &mut FnMut(Annotatable),
) {
    if let Some(model) = Model::from_annotable(cx, span, annotatable) {
        let options = changeset_options(cx, meta_item).unwrap();
        push(Annotatable::Item(changeset_impl(cx, span, &options, &model).unwrap()));
    } else {
        cx.span_err(meta_item.span,
            "`changeset_for` may only be applied to enums and structs");
    }
}

struct ChangesetOptions {
    table_name: ast::Ident,
    treat_none_as_null: Vec<TokenTree>,
}

fn changeset_options(cx: &mut ExtCtxt, meta_item: &MetaItem) -> Result<ChangesetOptions, ()> {
    match meta_item.node {
        MetaItemKind::List(_, ref meta_items) => {
            let table_name = try!(table_name(cx, &meta_items[0]));
            let treat_none_as_null = try!(boolean_option(cx, &meta_items[1..], "treat_none_as_null"));
            Ok(ChangesetOptions {
                table_name: str_to_ident(&table_name),
                treat_none_as_null: treat_none_as_null,
            })
        }
        _ => usage_error(cx, meta_item.span()),
    }
}

fn table_name(cx: &mut ExtCtxt, meta_item: &NestedMetaItem) -> Result<InternedString, ()> {
    match meta_item.word() {
        Some(word) => Ok(word.name().clone()),
        _ => usage_error(cx, meta_item.span()),
    }
}

fn boolean_option(cx: &mut ExtCtxt, meta_items: &[NestedMetaItem], option_name: &str)
    -> Result<Option<bool>, ()>
{
    if let Some(item) = meta_items.iter().find(|item| item.check_name(option_name)) {
        match item.value_str() {
            Some(ref s) if *s == "true" => Ok(quote_tokens!(cx, "true")),
            Some(ref s) if *s == "false" => Ok(quote_tokens!(cx, "false")),
            _ => {
                cx.span_err(item.span,
                    &format!("Expected {} to be in the form `option=\"true\"` or \
                            option=\"false\"", option_name));
                Err(())
            }
        }
    } else {
        Ok(quote_tokens!(cx, "false"))
    }
}

fn usage_error<T>(cx: &mut ExtCtxt, span: Span) -> Result<T, ()> {
    cx.span_err(span,
        "`changeset_for` must be used in the form `#[changeset_for(table1)]`");
    Err(())
}

fn changeset_impl(
    cx: &mut ExtCtxt,
    span: Span,
    options: &ChangesetOptions,
    model: &Model,
) -> Option<P<ast::Item>> {
    let struct_name = model.name;
    let table_name = options.table_name;
    let treat_none_as_null = &options.treat_none_as_null;
    let struct_ty = &model.ty;
    let lifetimes = lifetime_list_tokens(&model.generics.lifetimes, span);

    let pk = model.primary_key_name();
    let fields = model.attrs.iter()
        .filter(|a| a.column_name.name != pk.name)
        .map(|a| a.to_stable_macro_tokens(cx))
        .collect::<Vec<_>>();

    quote_item!(cx, AsChangeset! {
        (
            struct_name = $struct_name,
            table_name = $table_name,
            treat_none_as_null = $treat_none_as_null,
            struct_ty = $struct_ty,
            lifetimes = ($lifetimes),
        ),
        fields = [$fields],
    })
}
