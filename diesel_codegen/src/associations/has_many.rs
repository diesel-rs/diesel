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
        for item in selectable_column_hack(&builder).into_iter() {
            push(Annotatable::Item(item));
        }
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
        to_foreign_key(&self.model.name.name.as_str())
    }

    fn foreign_key(&self) -> ast::Path {
        self.cx.path(self.span, vec![self.association_name(), self.foreign_key_name()])
    }

    fn column_path(&self, column_name: ast::Ident) -> ast::Path {
        self.cx.path(self.span, vec![self.table_name(), column_name])
    }
}

fn join_to_impl(builder: &HasManyAssociationBuilder) -> P<ast::Item> {
    let foreign_table = builder.foreign_table();
    let table = builder.table();
    let foreign_key = builder.foreign_key();

    quote_item!(builder.cx,
        impl ::diesel::JoinTo<$foreign_table> for $table {
            fn join_sql(&self, out: &mut ::diesel::query_builder::QueryBuilder)
                -> ::diesel::query_builder::BuildQueryResult
            {
                try!($foreign_table.from_clause(out));
                out.push_sql(" ON ");
                ::diesel::query_builder::QueryFragment::to_sql(
                    &$foreign_key.nullable().eq($table.primary_key().nullable()),
                    out,
                )
            }
        }
    ).unwrap()
}

fn selectable_column_hack(builder: &HasManyAssociationBuilder) -> Vec<P<ast::Item>> {
    let mut result = builder.model.attrs.iter().flat_map(|attr| {
        selectable_column_impl(builder, attr.column_name)
    }).collect::<Vec<_>>();
    result.append(&mut selectable_column_impl(builder, str_to_ident("star")));
    result
}

fn selectable_column_impl(
    builder: &HasManyAssociationBuilder,
    column_name: ast::Ident,
) -> Vec<P<ast::Item>> {
    let table = builder.table();
    let foreign_table = builder.foreign_table();
    let column = builder.column_path(column_name);

    [quote_item!(builder.cx,
        impl ::diesel::expression::SelectableExpression<
            ::diesel::query_source::InnerJoinSource<$table, $foreign_table>
        > for $column {}
    ).unwrap(), quote_item!(builder.cx,
        impl ::diesel::expression::SelectableExpression<
            ::diesel::query_source::InnerJoinSource<$foreign_table, $table>
        > for $column {}
    ).unwrap(), quote_item!(builder.cx,
        impl ::diesel::expression::SelectableExpression<
            ::diesel::query_source::LeftOuterJoinSource<$table, $foreign_table>,
        > for $column {}
    ).unwrap(), quote_item!(builder.cx,
        impl ::diesel::expression::SelectableExpression<
            ::diesel::query_source::LeftOuterJoinSource<$foreign_table, $table>,
            <<$column as ::diesel::Expression>::SqlType
                as ::diesel::types::IntoNullable>::Nullable,
        > for $column {}
    ).unwrap()].to_vec()
}
