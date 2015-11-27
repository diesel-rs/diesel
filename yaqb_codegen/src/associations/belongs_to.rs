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
use super::{parse_association_options, AssociationOptions, to_foreign_key};

pub fn expand_belongs_to(
    cx: &mut ExtCtxt,
    span: Span,
    meta_item: &MetaItem,
    annotatable: &Annotatable,
    push: &mut FnMut(Annotatable)
) {
    let options = parse_association_options("belongs_to", cx, span, meta_item, annotatable);
    if let Some((builder, model, options)) = options {
        let builder = BelongsToAssociationBuilder {
            builder: builder,
            model: model,
            options: options,
        };

        push(Annotatable::Item(belonging_to_dsl_impl(cx, &builder)));
    }
}

struct BelongsToAssociationBuilder {
    pub options: AssociationOptions,
    pub model: Model,
    builder: aster::AstBuilder,
}

impl BelongsToAssociationBuilder {
    fn parent_struct_name(&self) -> ast::Ident {
        let association_name = self.options.name.name.as_str();
        let struct_name = association_name[..1].to_uppercase() + &association_name[1..];
        str_to_ident(&struct_name)
    }

    fn child_struct_name(&self) -> ast::Ident {
        self.model.name
    }

    fn child_table_name(&self) -> ast::Ident {
        self.model.table_name()
    }

    fn child_table(&self) -> ast::Path {
        self.builder.path()
            .segment(self.child_table_name()).build()
            .segment("table").build()
            .build()
    }

    fn foreign_key_name(&self) -> ast::Ident {
        to_foreign_key(&self.options.name.name.as_str())
    }

    fn foreign_key(&self) -> ast::Path {
        self.builder.path()
            .segment(self.child_table_name()).build()
            .segment(self.foreign_key_name()).build()
            .build()
    }

    fn foreign_key_type(&self) -> P<ast::Ty> {
        self.model.attr_named(self.foreign_key_name())
            .ty.clone()
    }

    fn primary_key_name(&self) -> ast::Ident {
        str_to_ident("id")
    }
}

fn belonging_to_dsl_impl(
    cx: &mut ExtCtxt,
    builder: &BelongsToAssociationBuilder,
) -> P<ast::Item> {
    let parent_struct_name = builder.parent_struct_name();
    let child_struct_name = builder.child_struct_name();
    let child_table = builder.child_table();
    let foreign_key = builder.foreign_key();
    let foreign_key_type = builder.foreign_key_type();
    let primary_key_name = builder.primary_key_name();

    quote_item!(cx,
        impl ::yaqb::BelongingToDsl<$parent_struct_name> for $child_struct_name {
            type Output = ::yaqb::helper_types::FindBy<
                $child_table,
                $foreign_key,
                $foreign_key_type,
            >;

            fn belonging_to(model: &$parent_struct_name) -> Self::Output {
                $child_table.filter($foreign_key.eq(model.$primary_key_name.clone()))
            }
        }
    ).unwrap()
}
