use syntax::ast;
use syntax::codemap::Span;
use syntax::ext::base::{Annotatable, ExtCtxt};
use syntax::parse::token;

use model::Model;
use util::{lifetime_list_tokens, comma_delimited_tokens};

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
        let primary_key_names = model.primary_key_names();
        let fields = model.field_tokens_for_stable_macro(cx);
        for name in primary_key_names {
            if model.attr_named(*name).is_none() {
                cx.span_err(span, &format!("Could not find a field named `{}` on `{}`", name, model.name));
                return;
            }
        }

        let primary_key_names = comma_delimited_tokens(
            primary_key_names.into_iter().map(|n| token::Ident(*n)), span);
        push(Annotatable::Item(quote_item!(cx, impl_Identifiable! {
            (
                table_name = $table_name,
                primary_key_names = ($primary_key_names),
                struct_ty = $struct_ty,
                lifetimes = ($lifetimes),
            ),
            fields = [$fields],
        }).unwrap()));
    }
}
