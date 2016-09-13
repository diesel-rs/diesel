use syntax::ast::{self, MetaItem, MetaItemKind};
use syntax::attr::AttrMetaMethods;
use syntax::codemap::Span;
use syntax::ext::base::{Annotatable, ExtCtxt};
use syntax::parse::token::str_to_ident;

use model::{infer_association_name, Model};

mod has_many;
mod belongs_to;

use self::has_many::expand_has_many;
use self::belongs_to::expand_belongs_to;

pub fn expand_derive_associations(
    cx: &mut ExtCtxt,
    span: Span,
    _: &MetaItem,
    annotatable: &Annotatable,
    push: &mut FnMut(Annotatable)
) {
    for attr in annotatable.attrs() {
        if attr.check_name("has_many") {
            expand_has_many(cx, span, &attr.node.value, annotatable, push);
        }

        if attr.check_name("belongs_to") {
            expand_belongs_to(cx, span, &attr.node.value, annotatable, push);
        }
    }
}

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

struct AssociationOptions {
    name: ast::Ident,
    foreign_key_name: Option<ast::Ident>,
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
            let foreign_key_name = options.iter().find(|a| a.check_name("foreign_key"))
                .and_then(|a| a.value_str()).map(|s| str_to_ident(&s));

            Some(AssociationOptions {
                name: association_name,
                foreign_key_name: foreign_key_name,
            })
        }
        _ => usage_err(),
    }
}

fn to_foreign_key(model_name: &str) -> ast::Ident {
    let lower_cased = infer_association_name(model_name);
    str_to_ident(&format!("{}_id", &lower_cased))
}

#[test]
fn to_foreign_key_properly_handles_underscores() {
    assert_eq!(str_to_ident("foo_bar_id"), to_foreign_key("FooBar"));
    assert_eq!(str_to_ident("foo_bar_baz_id"), to_foreign_key("FooBarBaz"));
}
