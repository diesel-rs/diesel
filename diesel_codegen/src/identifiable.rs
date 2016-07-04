use syntax::ast;
use syntax::codemap::Span;
use syntax::ext::base::{Annotatable, ExtCtxt};

use model::Model;

pub fn expand_derive_identifiable(
    cx: &mut ExtCtxt,
    span: Span,
    _meta_item: &ast::MetaItem,
    annotatable: &Annotatable,
    push: &mut FnMut(Annotatable)
) {
    if let Some(model) = Model::from_annotable(cx, span, annotatable) {
        let struct_name = model.name;
        let table_name = model.table_name();
        let primary_key_name = model.primary_key_name();
        let primary_key_type = match model.attr_named(primary_key_name) {
            Some(a) => a.ty.clone(),
            None => {
                let err_msg = format!(
                    "Could not find a field named `{}` on `{}`",
                    primary_key_name,
                    struct_name,
                );
                cx.span_err(span, &err_msg);
                return;
            }
        };

        let item = quote_item!(cx,
            impl ::diesel::associations::Identifiable for $struct_name {
                type Id = $primary_key_type;
                type Table = $table_name::table;

                fn table() -> Self::Table {
                    $table_name::table
                }

                fn id(&self) -> Self::Id {
                    self.$primary_key_name
                }
            }
        ).unwrap();
        push(Annotatable::Item(item));
    }
}
