use syntax::ast::{
    self,
    MetaItem,
};
use syntax::codemap::Span;
use syntax::ext::base::{Annotatable, ExtCtxt};
use syntax::ext::build::AstBuilder;
use syntax::parse::token::str_to_ident;
use syntax::ptr::P;
use inflector::Inflector;

use model::Model;
use super::{parse_association_options, AssociationOptions, to_foreign_key};
use util::{ty_param_of_option, is_option_ty};

pub fn expand_belongs_to(
    cx: &mut ExtCtxt,
    span: Span,
    meta_item: &MetaItem,
    annotatable: &Annotatable,
    push: &mut FnMut(Annotatable),
) {
    let options = parse_association_options("belongs_to", cx, span, meta_item, annotatable);
    if let Some((model, options)) = options {
        let builder = BelongsToAssociationBuilder {
            model: model,
            options: options,
            cx: cx,
            span: span,
        };

        belonging_to_dsl_impl(&builder, push);
        push(Annotatable::Item(join_to_impl(&builder)));
        if let Some(item) = belongs_to_impl(&builder) {
            push(Annotatable::Item(item));
        }
    }
}

struct BelongsToAssociationBuilder<'a, 'b: 'a> {
    pub options: AssociationOptions,
    pub model: Model,
    pub cx: &'a mut ExtCtxt<'b>,
    pub span: Span,
}

impl<'a, 'b> BelongsToAssociationBuilder<'a, 'b> {
    fn parent_struct_name(&self) -> ast::Ident {
        let association_name = self.options.name.name.as_str();
        str_to_ident(&association_name.to_class_case())
    }

    fn child_struct_name(&self) -> ast::Ident {
        self.model.name
    }

    fn child_table_name(&self) -> ast::Ident {
        self.model.table_name()
    }

    fn child_table(&self) -> ast::Path {
        self.cx.path(self.span, vec![self.child_table_name(), str_to_ident("table")])
    }

    fn parent_table_name(&self) -> ast::Ident {
        let pluralized = self.options.name.name.as_str().to_plural();
        str_to_ident(&pluralized)
    }

    fn parent_table(&self) -> ast::Path {
        self.cx.path(self.span, vec![self.parent_table_name(), str_to_ident("table")])
    }

    fn foreign_key_name(&self) -> ast::Ident {
        self.options.fk.unwrap_or(to_foreign_key(&self.options.name.name.as_str()))
    }

    fn foreign_key(&self) -> ast::Path {
        self.cx.path(self.span, vec![self.child_table_name(), self.foreign_key_name()])
    }

    fn foreign_key_type(&self) -> P<ast::Ty> {
        let name = self.foreign_key_name();
        self.model.attr_named(name)
            .expect(&format!("Couldn't find an attr named {}", name))
            .ty.clone()
    }

    fn primary_key_name(&self) -> ast::Ident {
        str_to_ident("id")
    }

    fn primary_key_type(&self) -> P<ast::Ty> {
        let ty = self.foreign_key_type();
        ty_param_of_option(&ty).map(|t| t.clone())
            .unwrap_or(ty)
    }
}

fn belonging_to_dsl_impl(
    builder: &BelongsToAssociationBuilder,
    push: &mut FnMut(Annotatable),
) {
    let parent_struct_name = builder.parent_struct_name();
    let child_struct_name = builder.child_struct_name();
    let child_table = builder.child_table();
    let foreign_key = builder.foreign_key();
    let primary_key_type = builder.primary_key_type();
    let primary_key_name = builder.primary_key_name();

    let item = quote_item!(builder.cx,
        impl ::diesel::BelongingToDsl<$parent_struct_name, $foreign_key> for $child_struct_name {
            type Output = ::diesel::helper_types::FindBy<
                $child_table,
                $foreign_key,
                $primary_key_type,
            >;

            fn belonging_to(model: &$parent_struct_name) -> Self::Output {
                $child_table.filter($foreign_key.eq(model.$primary_key_name.clone()))
            }
        }
    ).unwrap();
    push(Annotatable::Item(item));

    let item = quote_item!(builder.cx,
        impl ::diesel::BelongingToDsl<Vec<$parent_struct_name>, $foreign_key> for $child_struct_name {
            type Output = ::diesel::helper_types::Filter<
                $child_table,
                ::diesel::expression::helper_types::EqAny<
                    $foreign_key,
                    Vec<$primary_key_type>,
                >,
            >;

            fn belonging_to(parents: &Vec<$parent_struct_name>) -> Self::Output {
                let ids = parents.iter().map(|p| p.$primary_key_name.clone()).collect::<Vec<_>>();
                $child_table.filter($foreign_key.eq_any(ids))
            }
        }
    ).unwrap();
    push(Annotatable::Item(item));

    let item = quote_item!(builder.cx,
        impl ::diesel::BelongingToDsl<[$parent_struct_name], $foreign_key> for $child_struct_name {
            type Output = ::diesel::helper_types::Filter<
                $child_table,
                ::diesel::expression::helper_types::EqAny<
                    $foreign_key,
                    Vec<$primary_key_type>,
                >,
            >;

            fn belonging_to(parents: &[$parent_struct_name]) -> Self::Output {
                let ids = parents.iter().map(|p| p.$primary_key_name.clone()).collect::<Vec<_>>();
                $child_table.filter($foreign_key.eq_any(ids))
            }
        }
    ).unwrap();
    push(Annotatable::Item(item));
}

fn belongs_to_impl(builder: &BelongsToAssociationBuilder) -> Option<P<ast::Item>> {
    let parent_struct_name = builder.parent_struct_name();
    let child_struct_name = builder.child_struct_name();
    let primary_key_type = builder.primary_key_type();
    let foreign_key_name = builder.foreign_key_name();
    let foreign_key = builder.foreign_key();

    if is_option_ty(&builder.foreign_key_type()) {
        None
    } else {
        Some(quote_item!(builder.cx,
            impl ::diesel::associations::BelongsTo<$parent_struct_name, $foreign_key> for $child_struct_name {
                fn foreign_key(&self) -> $primary_key_type {
                    self.$foreign_key_name
                }
            }
        ).unwrap())
    }
}

fn join_to_impl(builder: &BelongsToAssociationBuilder) -> P<ast::Item> {
    let child_table = builder.child_table();
    let parent_table = builder.parent_table();
    let foreign_key = builder.foreign_key();

    quote_item!(builder.cx,
        joinable_inner!($child_table => $parent_table : ($foreign_key = $parent_table = $child_table));
    ).unwrap()
}

