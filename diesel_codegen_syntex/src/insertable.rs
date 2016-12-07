use syntax::ast::{self, MetaItem};
use syntax::codemap::Span;
use syntax::ext::base::{Annotatable, ExtCtxt};
use syntax::ptr::P;

use model::Model;
use util::{lifetime_list_tokens, struct_ty};

pub fn expand_derive_insertable(
    cx: &mut ExtCtxt,
    span: Span,
    _meta_item: &MetaItem,
    annotatable: &Annotatable,
    push: &mut FnMut(Annotatable)
) {
    if let Some(model) = Model::from_annotable(cx, span, annotatable) {
        insertable_impl(cx, span, model.table_name(), &model)
            .map(Annotatable::Item)
            .map(push);
    }
}

#[allow(unused_imports)]
fn insertable_impl(
    cx: &mut ExtCtxt,
    span: Span,
    table_name: ast::Ident,
    model: &Model,
) -> Option<P<ast::Item>> {
    if !model.generics.ty_params.is_empty() {
        cx.span_err(span, "#[derive(Insertable)] does not support generic types");
        return None;
    }

    let struct_name = model.name;
    let ty = struct_ty(cx, span, struct_name, &model.generics);

    let lifetimes = lifetime_list_tokens(&model.generics.lifetimes, span);
    let fields = model.attrs.iter().map(|a| a.to_stable_macro_tokens(cx)).collect::<Vec<_>>();

    quote_item!(cx, impl_Insertable! {
        (
            struct_name = $struct_name,
            table_name = $table_name,
            struct_ty = $ty,
            lifetimes = ($lifetimes),
        ),
        fields = [$fields],
    })
}
