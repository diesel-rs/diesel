use aster;
use syntax::ast::{
    self,
    Item,
    MetaItem,
};
use syntax::codemap::Span;
use syntax::ext::base::{Annotatable, ExtCtxt};
use syntax::parse::token::str_to_ident;
use syntax::ptr::P;

use model::Model;
use attr::Attr;

mod has_many;
mod belongs_to;

pub use self::has_many::expand_has_many;
pub use self::belongs_to::expand_belongs_to;

fn parse_association_options(
    association_kind: &str,
    cx: &mut ExtCtxt,
    span: Span,
    meta_item: &MetaItem,
    annotatable: &Annotatable,
) -> Option<(aster::AstBuilder, Model, AssociationOptions)> {
    let builder = aster::AstBuilder::new().span(span);
    let model = match Model::from_annotable(cx, &builder, annotatable) {
        Some(model) => model,
        None => {
            cx.span_err(span,
                &format!("#[{}] can only be applied to structs or tuple structs",
                         association_kind));
            return None;
        }
    };

    build_association_options(association_kind, cx, span, meta_item).map(|options| {
        (builder, model, options)
    })
}

struct AssociationOptions {
    name: ast::Ident,
}

fn build_association_options(
    association_kind: &str,
    cx: &mut ExtCtxt,
    span: Span,
    meta_item: &MetaItem,
) -> Option<AssociationOptions> {
    let usage_err = || {
        cx.span_err(span,
            &format!("`#[{}]` must be in the form `#[{}(table_name, option=value)]`",
                     association_kind, association_kind));
        None
    };
    match meta_item.node {
        ast::MetaList(_, ref options) => {
            let association_name = match options[0].node {
                ast::MetaWord(ref name) => str_to_ident(&name),
                _ => return usage_err(),
            };

            Some(AssociationOptions {
                name: association_name,
            })
        }
        _ => usage_err(),
    }
}

fn to_foreign_key(model_name: &str) -> ast::Ident {
    let lower_cased = model_name.to_lowercase();
    str_to_ident(&format!("{}_id", &lower_cased))
}
