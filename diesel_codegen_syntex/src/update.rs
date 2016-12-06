use syntax::ast::{self, MetaItem, NestedMetaItem, MetaItemKind};
use syntax::attr::HasAttrs;
use syntax::codemap::Span;
use syntax::ext::base::{Annotatable, ExtCtxt};
use syntax::ptr::P;

use model::Model;
use util::{lifetime_list_tokens, str_value_of_attr_with_name};

pub fn expand_derive_as_changeset(
    cx: &mut ExtCtxt,
    span: Span,
    meta_item: &MetaItem,
    annotatable: &Annotatable,
    push: &mut FnMut(Annotatable),
) {
    if let Some(model) = Model::from_annotable(cx, span, annotatable) {
        let options = changeset_options(cx, span, annotatable.attrs()).unwrap();
        push(Annotatable::Item(changeset_impl(cx, span, &options, &model).unwrap()));
    } else {
        cx.span_err(meta_item.span,
            "`#[derive(AsChangeset)]` may only be applied to enums and structs");
    }
}

struct ChangesetOptions {
    table_name: ast::Ident,
    treat_none_as_null: bool,
}

fn changeset_options(
    cx: &mut ExtCtxt,
    span: Span,
    attributes: &[ast::Attribute]
) -> Result<ChangesetOptions, ()> {
    let changeset_options_attr = attributes.iter().find(|a| a.check_name("changeset_options"));
    let treat_none_as_null = try!(changeset_options_attr
        .map(|a| extract_treat_none_as_null(cx, a))
        .unwrap_or(Ok(false)));
    let table_name = match str_value_of_attr_with_name(cx, attributes, "table_name") {
        Some(name) => name,
        None => return missing_table_name_error(cx, span),
    };

    Ok(ChangesetOptions {
        table_name: table_name,
        treat_none_as_null: treat_none_as_null,
    })
}

fn extract_treat_none_as_null(cx: &mut ExtCtxt, attr: &ast::Attribute) -> Result<bool, ()>{
    match attr.node.value.node {
        MetaItemKind::List(_, ref items) if items.len() == 1 => {
            if items[0].check_name("treat_none_as_null") {
                boolean_option(cx, &items[0])
            } else {
                options_usage_error(cx, attr.span)
            }
        }
        _ => options_usage_error(cx, attr.span),
    }
}

fn boolean_option(cx: &mut ExtCtxt, item: &NestedMetaItem) -> Result<bool, ()> {
    match item.value_str() {
        Some(ref s) if *s == "true" => Ok(true),
        Some(ref s) if *s == "false" => Ok(false),
        _ => options_usage_error(cx, item.span())
    }
}

fn options_usage_error<T>(cx: &mut ExtCtxt, span: Span) -> Result<T, ()> {
    cx.span_err(span,
        r#"`changeset_options` must be used in the form \
        `#[changeset_options(treat_none_as_null = "true")]`"#);
    Err(())
}

fn missing_table_name_error<T>(cx: &mut ExtCtxt, span: Span) -> Result<T, ()> {
    cx.span_err(span, r#"Structs annotated with `#[derive(AsChangeset)]` must \
        also be annotated with `#[table_name="something"]`"#);
    Err(())
}

#[allow(unused_imports)] // quote_tokens! generates warnings
fn changeset_impl(
    cx: &mut ExtCtxt,
    span: Span,
    options: &ChangesetOptions,
    model: &Model,
) -> Option<P<ast::Item>> {
    let struct_name = model.name;
    let table_name = options.table_name;
    let treat_none_as_null = if options.treat_none_as_null {
        quote_tokens!(cx, "true")
    } else {
        quote_tokens!(cx, "false")
    };
    let struct_ty = &model.ty;
    let lifetimes = lifetime_list_tokens(&model.generics.lifetimes, span);

    let pk = model.primary_key_name();
    let fields = model.attrs.iter()
        .filter(|a| a.column_name.name != pk.name)
        .map(|a| a.to_stable_macro_tokens(cx))
        .collect::<Vec<_>>();

    quote_item!(cx, impl_AsChangeset! {
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
