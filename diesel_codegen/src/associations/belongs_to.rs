use aster;
use syntax::ast::{
    self,
    MetaItem,
};
use syntax::codemap::Span;
use syntax::ext::base::{Annotatable, ExtCtxt};
use syntax::parse::token::str_to_ident;
use syntax::ptr::P;

use model::Model;
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
        push(Annotatable::Item(join_to_impl(cx, &builder)));
        for item in selectable_column_hack(cx, &builder).into_iter() {
            push(Annotatable::Item(item));
        }
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
        self.builder.path()
            .segment(self.child_table_name()).build()
            .segment("table").build()
            .build()
    }

    fn parent_table_name(&self) -> ast::Ident {
        let pluralized = format!("{}s", &self.options.name.name.as_str());
        str_to_ident(&pluralized)
    }

    fn parent_table(&self) -> ast::Path {
        self.builder.path()
            .segment(self.parent_table_name()).build()
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

fn capitalize_from_association_name(name: String) -> String {
    let mut result = String::with_capacity(name.len());
    let words = name.split("_");

    for word in words {
        result.push_str(&word[..1].to_uppercase());
        result.push_str(&word[1..]);
    }

    result
}

impl ::std::ops::Deref for BelongsToAssociationBuilder {
    type Target = aster::AstBuilder;

    fn deref(&self) -> &Self::Target {
        &self.builder
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
        impl ::diesel::BelongingToDsl<$parent_struct_name> for $child_struct_name {
            type Output = ::diesel::helper_types::FindBy<
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

fn join_to_impl(
    cx: &mut ExtCtxt,
    builder: &BelongsToAssociationBuilder,
) -> P<ast::Item> {
    let child_table = builder.child_table();
    let parent_table = builder.parent_table();
    let foreign_key = builder.foreign_key();

    quote_item!(cx,
        impl ::diesel::JoinTo<$parent_table> for $child_table {
            fn join_sql(&self, out: &mut ::diesel::query_builder::QueryBuilder)
                -> ::diesel::query_builder::BuildQueryResult
            {
                try!($parent_table.from_clause(out));
                out.push_sql(" ON ");
                $foreign_key.eq($parent_table.primary_key()).to_sql(out)
            }
        }
    ).unwrap()
}

fn selectable_column_hack(
    cx: &mut ExtCtxt,
    builder: &BelongsToAssociationBuilder,
) -> Vec<P<ast::Item>> {
    let mut result = builder.model.attrs.iter().flat_map(|attr| {
        selectable_column_impl(cx, builder, attr.column_name)
    }).collect::<Vec<_>>();
    result.append(&mut selectable_column_impl(cx, builder, str_to_ident("star")));
    result
}

fn selectable_column_impl(
    cx: &mut ExtCtxt,
    builder: &BelongsToAssociationBuilder,
    column_name: ast::Ident,
) -> Vec<P<ast::Item>> {
    let parent_table = builder.parent_table();
    let child_table = builder.child_table();
    let column = builder.path()
        .segment(builder.child_table_name()).build()
        .segment(column_name).build()
        .build();

    [quote_item!(cx,
        impl ::diesel::expression::SelectableExpression<
            ::diesel::query_source::InnerJoinSource<$parent_table, $child_table>
        > for $column {}
    ).unwrap(), quote_item!(cx,
        impl ::diesel::expression::SelectableExpression<
            ::diesel::query_source::InnerJoinSource<$child_table, $parent_table>
        > for $column {}
    ).unwrap(), quote_item!(cx,
        impl ::diesel::expression::SelectableExpression<
            ::diesel::query_source::LeftOuterJoinSource<$child_table, $parent_table>,
        > for $column {}
    ).unwrap(), quote_item!(cx,
        impl ::diesel::expression::SelectableExpression<
            ::diesel::query_source::LeftOuterJoinSource<$parent_table, $child_table>,
            <<$column as ::diesel::Expression>::SqlType
                as ::diesel::types::IntoNullable>::Nullable,
        > for $column {}
    ).unwrap()].to_vec()
}
