use syntax::ast;
use syntax::codemap::Span;
use syntax::ext::base::{Annotatable, ExtCtxt};

use model::Model;
use util::lifetime_list_tokens;

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
        let lifetimes = lifetime_list_tokens(&model.generics.lifetimes, span);
        let primary_key_name = model.primary_key_name;
        let fields = model.field_tokens_for_stable_macro(cx);
        if model.attr_named(primary_key_name).is_some() {
            push(Annotatable::Item(quote_item!(cx, impl_Identifiable! {
                (
                    table_name = $table_name,
                    primary_key_name = $primary_key_name,
                    struct_ty = $struct_ty,
                    lifetimes = ($lifetimes),
                ),
                fields = [$fields],
            }).unwrap()));
        } else {
            cx.span_err(span, &format!("Could not find a field named `{}` on `{}`", primary_key_name, model.name));
        }
    }
}
