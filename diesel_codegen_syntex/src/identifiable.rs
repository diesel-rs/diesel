use syntax::ast;
use syntax::codemap::Span;
use syntax::ext::base::{Annotatable, ExtCtxt};
use syntax::parse::token::str_to_ident;

use model::Model;

pub fn expand_derive_identifiable(
    cx: &mut ExtCtxt,
    span: Span,
    _meta_item: &ast::MetaItem,
    annotatable: &Annotatable,
    push: &mut FnMut(Annotatable)
) {
    if let Some(model) = Model::from_annotable(cx, span, annotatable) {
        let table_name = model.table_name();
        let struct_ty = &model.ty;
        let fields = model.field_tokens_for_stable_macro(cx);
        if model.attr_named(str_to_ident("id")).is_some() {
            push(Annotatable::Item(quote_item!(cx, Identifiable! {
                (
                    table_name = $table_name,
                    struct_ty = $struct_ty,
                ),
                fields = [$fields],
            }).unwrap()));
        } else {
            cx.span_err(span, &format!("Could not find a field named `id` on `{}`", model.name));
        }
    }
}
