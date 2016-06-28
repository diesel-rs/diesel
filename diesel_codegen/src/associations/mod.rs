use syntax::ast::{self, MetaItem, MetaItemKind};
use syntax::codemap::Span;
use syntax::ext::base::{Annotatable, ExtCtxt};
use syntax::parse::token::str_to_ident;
use inflector::Inflector;

use model::{infer_association_name, Model};

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
) -> Option<(Model, AssociationOptions)> {
    let model = match Model::from_annotable(cx, span, annotatable) {
        Some(model) => model,
        None => {
            cx.span_err(span,
                &format!("#[{}] can only be applied to structs or tuple structs",
                         association_kind));
            return None;
        }
    };

    build_association_options(association_kind, cx, span, meta_item).map(|options| {
        (model, options)
    })
}

#[derive(Debug)]
struct AssociationOptions {
    name: ast::Ident,
    fk: Option<ast::Ident>,
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
        MetaItemKind::List(_, ref options) => {
            let association_name = match options[0].node {
                MetaItemKind::Word(ref name) => str_to_ident(&name),
                _ => return usage_err(),
            };
            let fk = if options.len() >1 {
                match options[1].node {
                    MetaItemKind::Word(ref name) => Some(str_to_ident(&name)),
                    _ =>  None,
                }
            } else{
                None
            };


            Some(AssociationOptions {
                name: association_name,
                fk: fk,
            })
        }
        _ => usage_err(),
    }
}

fn to_foreign_key(model_name: &str) -> ast::Ident {
    let lower_cased = infer_association_name(model_name);
    str_to_ident(&lower_cased.to_foreign_key())
}

#[test]
fn to_foreign_key_properly_handles_underscores() {
    assert_eq!(str_to_ident("foo_bar_id"), to_foreign_key("FooBar"));
    assert_eq!(str_to_ident("foo_bar_baz_id"), to_foreign_key("FooBarBaz"));
}
