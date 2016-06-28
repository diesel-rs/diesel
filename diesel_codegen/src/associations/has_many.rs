use syntax::ast::{
    self,
    MetaItem,
};
use syntax::codemap::Span;
use syntax::ext::base::{Annotatable, ExtCtxt};
use syntax::ext::build::AstBuilder;
use syntax::ptr::P;
use syntax::parse::token::str_to_ident;

use model::Model;
use super::{parse_association_options, AssociationOptions, to_foreign_key};

pub fn expand_has_many(
    cx: &mut ExtCtxt,
    span: Span,
    meta_item: &MetaItem,
    annotatable: &Annotatable,
    push: &mut FnMut(Annotatable)
) {
    let options = parse_association_options("has_many", cx, span, meta_item, annotatable);
    if let Some((model, options)) = options {
        let builder = HasManyAssociationBuilder {
            options: options,
            model: model,
            cx: cx,
            span: span,
        };
        push(Annotatable::Item(join_to_impl(&builder)));
    }
}

struct HasManyAssociationBuilder<'a, 'b: 'a> {
    pub options: AssociationOptions,
    pub model: Model,
    pub cx: &'a mut ExtCtxt<'b>,
    pub span: Span,
}

impl<'a, 'b> HasManyAssociationBuilder<'a, 'b> {
    fn association_name(&self) -> ast::Ident {
        self.options.name
    }

    fn foreign_table(&self) -> ast::Path {
        self.cx.path(self.span, vec![self.association_name(), str_to_ident("table")])
    }

    fn table_name(&self) -> ast::Ident {
        self.model.table_name()
    }

    fn table(&self) -> ast::Path {
        self.cx.path(self.span, vec![self.table_name(), str_to_ident("table")])
    }

    fn foreign_key_name(&self) -> ast::Ident {
        self.options.fk.unwrap_or(to_foreign_key(&self.model.name.name.as_str()))
    }

    fn foreign_key(&self) -> ast::Path {
        self.cx.path(self.span, vec![self.association_name(), self.foreign_key_name()])
    }
}

fn join_to_impl(builder: &HasManyAssociationBuilder) -> P<ast::Item> {
    let foreign_table = builder.foreign_table();
    let table = builder.table();
    let foreign_key = builder.foreign_key();

    quote_item!(builder.cx,
        joinable_inner!($table => $foreign_table : ($foreign_key = $table = $foreign_table));
    ).unwrap()
}
