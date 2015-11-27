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

pub fn expand_has_many(
    cx: &mut ExtCtxt,
    span: Span,
    meta_item: &MetaItem,
    annotatable: &Annotatable,
    push: &mut FnMut(Annotatable)
) {
    let builder = aster::AstBuilder::new().span(span);
    let model = match Model::from_annotable(cx, &builder, annotatable) {
        Some(model) => model,
        None => {
            cx.span_err(span,
                "#[has_many] can only be applied to structs or tuple structs");
            return;
        }
    };

    if let Some(options) = build_has_many_options(cx, span, meta_item) {
        let builder = HasManyAssociationBuilder {
            options: options,
            model: model,
            builder: builder,
        };
        push(Annotatable::Item(has_many_method(cx, &builder)));
        push(Annotatable::Item(join_to_impl(cx, &builder)));
        for item in selectable_column_hack(cx, &builder).into_iter() {
            push(Annotatable::Item(item));
        }
    }
}

struct HasManyOptions {
    name: ast::Ident,
}

struct HasManyAssociationBuilder {
    pub options: HasManyOptions,
    pub model: Model,
    builder: aster::AstBuilder,
}

impl HasManyAssociationBuilder {
    fn struct_name(&self) -> &P<ast::Ty> {
        &self.model.ty
    }

    fn association_name(&self) -> ast::Ident {
        self.options.name
    }

    fn primary_key(&self) -> &Attr {
        self.model.primary_key()
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

fn build_has_many_options(
    cx: &mut ExtCtxt,
    span: Span,
    meta_item: &MetaItem,
) -> Option<HasManyOptions> {
    let usage_err = || {
        cx.span_err(span,
            "`#[has_many]` must be in the form `#[has_many(child_table, option=value)]`");
        None
    };
    match meta_item.node {
        ast::MetaList(_, ref options) => {
            let association_name = match options[0].node {
                ast::MetaWord(ref name) => str_to_ident(&name),
                _ => return usage_err(),
            };

            Some(HasManyOptions {
                name: association_name,
            })
        }
        _ => usage_err(),
    }
}

fn has_many_method(
    cx: &mut ExtCtxt,
    builder: &HasManyAssociationBuilder,
) -> P<ast::Item> {
    let struct_name = builder.struct_name();
    let association_name = builder.association_name();
    let primary_key = builder.primary_key();
    let foreign_table = builder.foreign_table();
    let foreign_key = builder.foreign_key();
    let ref pk_type = primary_key.ty;
    let pk_access = builder.expr()
        .field(primary_key.field_name.unwrap())
        .self_();

    quote_item!(cx,
        impl $struct_name {
            pub fn $association_name(&self) -> ::yaqb::helper_types::FindBy<
                $foreign_table,
                $foreign_key,
                $pk_type,
            > {
                $foreign_table.filter($foreign_key.eq($pk_access))
            }
        }
    ).unwrap()
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

    vec![quote_item!(cx,
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
    ).unwrap()]
}

fn to_foreign_key(model_name: &str) -> ast::Ident {
    let lower_cased = model_name.to_lowercase();
    str_to_ident(&format!("{}_id", &lower_cased))
}
