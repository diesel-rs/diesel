use syntax::ast::{
    self,
    MetaItem,
};
use syntax::codemap::Span;
use syntax::ext::base::{Annotatable, ExtCtxt};
use syntax::ext::build::AstBuilder;
use syntax::parse::token::str_to_ident;
use syntax::ptr::P;

use model::Model;
use super::{parse_association_options, AssociationOptions, to_foreign_key};
use util::ty_param_of_option;

pub fn expand_belongs_to(
    cx: &mut ExtCtxt,
    span: Span,
    meta_item: &MetaItem,
    annotatable: &Annotatable,
    push: &mut FnMut(Annotatable)
) {
    let options = parse_association_options("belongs_to", cx, span, meta_item, annotatable);
    if let Some((model, options)) = options {
        let builder = BelongsToAssociationBuilder {
            model: model,
            options: options,
            cx: cx,
            span: span,
        };

        push(Annotatable::Item(belonging_to_dsl_impl(&builder)));
        push(Annotatable::Item(join_to_impl(&builder)));
        for item in selectable_column_hack(&builder).into_iter() {
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
        let struct_name = capitalize_from_association_name(association_name.to_string());
        str_to_ident(&struct_name)
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
        let pluralized = format!("{}s", &self.options.name.name.as_str());
        str_to_ident(&pluralized)
    }

    fn parent_table(&self) -> ast::Path {
        self.cx.path(self.span, vec![self.parent_table_name(), str_to_ident("table")])
    }

    fn foreign_key_name(&self) -> ast::Ident {
        to_foreign_key(&self.options.name.name.as_str())
    }

    fn foreign_key(&self) -> ast::Path {
        self.cx.path(self.span, vec![self.child_table_name(), self.foreign_key_name()])
    }

    fn foreign_key_type(&self) -> P<ast::Ty> {
        self.model.attr_named(self.foreign_key_name())
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

    fn column_path(&self, column_name: ast::Ident) -> ast::Path {
        self.cx.path(self.span, vec![self.child_table_name(), column_name])
    }
}

fn capitalize_from_association_name(name: String) -> String {
    let mut result = String::with_capacity(name.len());
    let words = name.split("_");

    for word in words {
        result.push_str(&word[..1].to_uppercase());
        result.push_str(&word[1..]);
    }

    result
}

fn belonging_to_dsl_impl(builder: &BelongsToAssociationBuilder) -> P<ast::Item> {
    let parent_struct_name = builder.parent_struct_name();
    let child_struct_name = builder.child_struct_name();
    let child_table = builder.child_table();
    let foreign_key = builder.foreign_key();
    let primary_key_type = builder.primary_key_type();
    let primary_key_name = builder.primary_key_name();

    quote_item!(builder.cx,
        impl ::diesel::BelongingToDsl<$parent_struct_name> for $child_struct_name {
            type Output = ::diesel::helper_types::FindBy<
                $child_table,
                $foreign_key,
                $primary_key_type,
            >;

            fn belonging_to(model: &$parent_struct_name) -> Self::Output {
                $child_table.filter($foreign_key.eq(model.$primary_key_name.clone()))
            }
        }
    ).unwrap()
}

fn join_to_impl(builder: &BelongsToAssociationBuilder) -> P<ast::Item> {
    let child_table = builder.child_table();
    let parent_table = builder.parent_table();
    let foreign_key = builder.foreign_key();

    quote_item!(builder.cx,
        impl ::diesel::JoinTo<$parent_table> for $child_table {
            fn join_sql(&self, out: &mut ::diesel::query_builder::QueryBuilder)
                -> ::diesel::query_builder::BuildQueryResult
            {
                try!($parent_table.from_clause(out));
                out.push_sql(" ON ");
                ::diesel::query_builder::QueryFragment::to_sql(
                    &$foreign_key.nullable().eq($parent_table.primary_key().nullable()), out)
            }
        }
    ).unwrap()
}

fn selectable_column_hack(builder: &BelongsToAssociationBuilder) -> Vec<P<ast::Item>> {
    let mut result = builder.model.attrs.iter().flat_map(|attr| {
        selectable_column_impl(builder, attr.column_name)
    }).collect::<Vec<_>>();
    result.append(&mut selectable_column_impl(builder, str_to_ident("star")));
    result
}

fn selectable_column_impl(
    builder: &BelongsToAssociationBuilder,
    column_name: ast::Ident,
) -> Vec<P<ast::Item>> {
    let parent_table = builder.parent_table();
    let child_table = builder.child_table();
    let column = builder.column_path(column_name);

    [quote_item!(builder.cx,
        impl ::diesel::expression::SelectableExpression<
            ::diesel::query_source::InnerJoinSource<$parent_table, $child_table>
        > for $column {}
    ).unwrap(), quote_item!(builder.cx,
        impl ::diesel::expression::SelectableExpression<
            ::diesel::query_source::InnerJoinSource<$child_table, $parent_table>
        > for $column {}
    ).unwrap(), quote_item!(builder.cx,
        impl ::diesel::expression::SelectableExpression<
            ::diesel::query_source::LeftOuterJoinSource<$child_table, $parent_table>,
        > for $column {}
    ).unwrap(), quote_item!(builder.cx,
        impl ::diesel::expression::SelectableExpression<
            ::diesel::query_source::LeftOuterJoinSource<$parent_table, $child_table>,
            <<$column as ::diesel::Expression>::SqlType
                as ::diesel::types::IntoNullable>::Nullable,
        > for $column {}
    ).unwrap()].to_vec()
}
