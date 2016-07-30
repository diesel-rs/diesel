use syntax::ast::{
    self,
    Item,
    MetaItem,
    MetaItemKind,
};
use syntax::codemap::Span;
use syntax::ext::base::{Annotatable, ExtCtxt};
use syntax::ptr::P;
use syntax::parse::token::{InternedString, str_to_ident};

use attr::Attr;
use util::{lifetime_list_tokens, struct_ty};

pub fn expand_insert(
    cx: &mut ExtCtxt,
    span: Span,
    meta_item: &MetaItem,
    annotatable: &Annotatable,
    push: &mut FnMut(Annotatable)
) {
    if let Annotatable::Item(ref item) = *annotatable {
        let tables = insertable_tables(cx, meta_item);
        for body in tables.into_iter().filter_map(|t| insertable_impl(cx, span, t, item)) {
            push(Annotatable::Item(body));
        }
    } else {
        cx.span_err(meta_item.span,
                    "`insertable_into` may only be applied to enums and structs");
    };
}

fn insertable_tables(cx: &mut ExtCtxt, meta_item: &MetaItem) -> Vec<InternedString> {
    match meta_item.node {
        MetaItemKind::List(_, ref meta_items) => {
            meta_items.iter().map(|i| table_name(cx, i)).collect()
        }
        _ => usage_error(cx, meta_item),
    }
}

fn table_name(cx: &mut ExtCtxt, meta_item: &MetaItem) -> InternedString {
    match meta_item.node {
        MetaItemKind::Word(ref word) => word.clone(),
        _ => usage_error(cx, meta_item),
    }
}

fn usage_error(cx: &mut ExtCtxt, meta_item: &MetaItem) -> ! {
    cx.span_err(meta_item.span,
        "`insertable_into` must be used in the form `#[insertable_into(table1, table2)]`");
    panic!()
}

#[allow(unused_imports)]
fn insertable_impl(
    cx: &mut ExtCtxt,
    span: Span,
    table: InternedString,
    item: &Item,
) -> Option<P<ast::Item>> {
    let (generics, fields) = match Attr::from_item(cx, item) {
        Some(vals) => vals,
        None => {
            cx.span_err(item.span,
                        "Expected a struct or tuple struct for `#[insertable_into]`");
            return None;
        }
    };

    if !generics.ty_params.is_empty() {
        cx.span_err(item.span, "#[insertable_into] does not support generic types");
        return None;
    }

    let struct_name = item.ident;
    let ty = struct_ty(cx, span, item.ident, &generics);
    let table_name = str_to_ident(&table);

    let lifetimes = lifetime_list_tokens(&generics.lifetimes, span);
    let fields = fields.iter().map(|a| a.to_stable_macro_tokens(cx)).collect::<Vec<_>>();

    quote_item!(cx, Insertable! {
        (
            struct_name = $struct_name,
            table_name = $table_name,
            struct_ty = $ty,
            lifetimes = ($lifetimes),
        ),
        fields = [$fields],
    })
}
