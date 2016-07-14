use syntax::ast;
use syntax::codemap::Span;
use syntax::ext::base::{Annotatable, ExtCtxt};
use syntax::ptr::P;
use syntax::parse::token::str_to_ident;
use inflector::Inflector;

use attr::Attr;
use util::{str_value_of_attr_with_name, struct_ty};

#[derive(Debug)]
pub struct Model {
    pub ty: P<ast::Ty>,
    pub attrs: Vec<Attr>,
    pub name: ast::Ident,
    table_name_from_annotation: Option<ast::Ident>,
}

impl Model {
    pub fn from_annotable(
        cx: &mut ExtCtxt,
        span: Span,
        annotatable: &Annotatable,
    ) -> Option<Self> {
        if let Annotatable::Item(ref item) = *annotatable {
            let table_name_from_annotation =
                str_value_of_attr_with_name(cx, &item.attrs, "table_name");
            Attr::from_item(cx, item).map(|(generics, attrs)| {
                let ty = struct_ty(cx, span, item.ident, &generics);
                Model {
                    ty: ty,
                    attrs: attrs,
                    name: item.ident,
                    table_name_from_annotation: table_name_from_annotation,
                }
            })
        } else {
            None
        }
    }

    pub fn primary_key_name(&self) -> ast::Ident {
        str_to_ident("id")
    }

    pub fn table_name(&self) -> ast::Ident {
        self.table_name_from_annotation.unwrap_or_else(|| {
            str_to_ident(&infer_table_name(&self.name.name.as_str()))
        })
    }

    pub fn attr_named(&self, name: ast::Ident) -> Option<&Attr> {
        self.attrs.iter().find(|attr| {
            attr.field_name.map(|f| f.name) == Some(name.name)
        })
    }
}

pub fn infer_association_name(name: &str) -> String {
    name.to_snake_case()
}

fn infer_table_name(name: &str) -> String {
    infer_association_name(name).to_plural()
}

#[test]
fn infer_table_name_pluralizes_and_downcases() {
    assert_eq!("foos", &infer_table_name("Foo"));
    assert_eq!("bars", &infer_table_name("Bar"));
}

#[test]
fn infer_table_name_properly_handles_underscores() {
    assert_eq!("foo_bars", &infer_table_name("FooBar"));
    assert_eq!("foo_bar_bazs", &infer_table_name("FooBarBaz"));
}

#[test]
fn infer_table_name_handle_crude() {
    assert_eq!("geometries", &infer_table_name("Geometry"));
}
