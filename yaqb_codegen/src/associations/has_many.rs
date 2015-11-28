use aster;
use syntax::ast::{
    self,
    Item,
    MetaItem,
};
use syntax::codemap::Span;
use syntax::ext::base::{Annotatable, ExtCtxt};
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
    if let Some((builder, model, options)) = options {
        let builder = HasManyAssociationBuilder {
            options: options,
            model: model,
            builder: builder,
        };
        push(Annotatable::Item(join_to_impl(cx, &builder)));
        for item in selectable_column_hack(cx, &builder).into_iter() {
            push(Annotatable::Item(item));
        }
    }
}

struct HasManyAssociationBuilder {
    pub options: AssociationOptions,
    pub model: Model,
    builder: aster::AstBuilder,
}

impl HasManyAssociationBuilder {
    fn association_name(&self) -> ast::Ident {
        self.options.name
    }

    fn foreign_table(&self) -> ast::Path {
        self.builder.path()
            .segment(self.association_name()).build()
            .segment("table").build()
            .build()
    }

    fn table_name(&self) -> ast::Ident {
        self.model.table_name()
    }

    fn table(&self) -> ast::Path {
        self.builder.path()
            .segment(self.table_name()).build()
            .segment("table").build()
            .build()
    }

    fn foreign_key_name(&self) -> ast::Ident {
        to_foreign_key(&self.model.name.name.as_str())
    }

    fn foreign_key(&self) -> ast::Path {
        self.builder.path()
            .segment(self.association_name()).build()
            .segment(self.foreign_key_name()).build()
            .build()
    }
}

impl ::std::ops::Deref for HasManyAssociationBuilder {
    type Target = aster::AstBuilder;

    fn deref(&self) -> &Self::Target {
        &self.builder
    }
}

fn join_to_impl(
    cx: &mut ExtCtxt,
    builder: &HasManyAssociationBuilder,
) -> P<ast::Item> {
    let foreign_table = builder.foreign_table();
    let table = builder.table();
    let foreign_key = builder.foreign_key();

    quote_item!(cx,
        impl ::yaqb::JoinTo<$foreign_table> for $table {
            fn join_sql(&self, out: &mut ::yaqb::query_builder::QueryBuilder)
                -> ::yaqb::query_builder::BuildQueryResult
            {
                try!($foreign_table.from_clause(out));
                out.push_sql(" ON ");
                $foreign_key.eq($table.primary_key()).to_sql(out)
            }
        }
    ).unwrap()
}

fn selectable_column_hack(
    cx: &mut ExtCtxt,
    builder: &HasManyAssociationBuilder,
) -> Vec<P<ast::Item>> {
    let mut result = builder.model.attrs.iter().flat_map(|attr| {
        selectable_column_impl(cx, builder, attr.column_name)
    }).collect::<Vec<_>>();
    result.append(&mut selectable_column_impl(cx, builder, str_to_ident("star")));
    result
}

fn selectable_column_impl(
    cx: &mut ExtCtxt,
    builder: &HasManyAssociationBuilder,
    column_name: ast::Ident,
) -> Vec<P<ast::Item>> {
    let table = builder.table();
    let foreign_table = builder.foreign_table();
    let column = builder.path()
        .segment(builder.table_name()).build()
        .segment(column_name).build()
        .build();

    [quote_item!(cx,
        impl ::yaqb::expression::SelectableExpression<
            ::yaqb::query_source::InnerJoinSource<$table, $foreign_table>
        > for $column {}
    ).unwrap(), quote_item!(cx,
        impl ::yaqb::expression::SelectableExpression<
            ::yaqb::query_source::InnerJoinSource<$foreign_table, $table>
        > for $column {}
    ).unwrap(), quote_item!(cx,
        impl ::yaqb::expression::SelectableExpression<
            ::yaqb::query_source::LeftOuterJoinSource<$table, $foreign_table>,
        > for $column {}
    ).unwrap(), quote_item!(cx,
        impl ::yaqb::expression::SelectableExpression<
            ::yaqb::query_source::LeftOuterJoinSource<$foreign_table, $table>,
            ::yaqb::types::Nullable<<$column as ::yaqb::Expression>::SqlType>,
        > for $column {}
    ).unwrap()].to_vec()
}
