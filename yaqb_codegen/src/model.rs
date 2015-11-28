use aster;
use syntax::ast;
use syntax::ext::base::{Annotatable, ExtCtxt};
use syntax::ptr::P;
use syntax::parse::token::str_to_ident;

use attr::Attr;

pub struct Model {
    pub ty: P<ast::Ty>,
    pub attrs: Vec<Attr>,
    pub name: ast::Ident,
}

impl Model {
    pub fn from_annotable(
        cx: &mut ExtCtxt,
        builder: &aster::AstBuilder,
        annotatable: &Annotatable,
    ) -> Option<Self> {
        if let Annotatable::Item(ref item) = *annotatable {
            Attr::from_item(cx, item).map(|(generics, attrs)| {
                let ty = builder.ty().path()
                    .segment(item.ident).with_generics(generics.clone())
                    .build().build();
                Model {
                    ty: ty,
                    attrs: attrs,
                    name: item.ident,
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
        let pluralized = format!("{}s", self.name.name.as_str());
        str_to_ident(&pluralized.to_lowercase())
    }

    pub fn attr_named(&self, name: ast::Ident) -> &Attr {
        self.attrs.iter().find(|attr| {
            attr.field_name == Some(name)
        }).expect(&format!("Couldn't find an attr named {}", name))
    }
}
