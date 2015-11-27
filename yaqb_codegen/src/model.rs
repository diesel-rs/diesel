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
            let ty = builder.ty().id(item.ident);
            Attr::from_item(cx, item).map(|(_, attrs)| {
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

    pub fn primary_key(&self) -> &Attr {
        self.attrs.iter().find(|attr| {
            attr.field_name == Some(str_to_ident("id"))
        }).expect("primary key must be named `id` for now")
    }

    pub fn table_name(&self) -> ast::Ident {
        let pluralized = format!("{}s", self.name.name.as_str());
        str_to_ident(&pluralized.to_lowercase())
    }
}
