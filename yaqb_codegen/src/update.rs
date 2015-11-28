use aster;
use syntax::ast::{self, MetaItem};
use syntax::codemap::Span;
use syntax::ext::base::{Annotatable, ExtCtxt};
use syntax::ptr::P;
use syntax::parse::token::InternedString;

use attr::Attr;
use model::Model;

pub fn expand_changeset_for(
    cx: &mut ExtCtxt,
    span: Span,
    meta_item: &MetaItem,
    annotatable: &Annotatable,
    push: &mut FnMut(Annotatable),
) {
    let builder = aster::AstBuilder::new().span(span);

    if let Some(model) = Model::from_annotable(cx, &builder, annotatable) {
        let table = changeset_tables(cx, meta_item).unwrap();
        push(Annotatable::Item(changeset_impl(cx, builder, &table, &model).unwrap()));
        if let Some(item) = save_changes_impl(cx, builder, &table, &model) {
            push(Annotatable::Item(item));
        }
    } else {
        cx.span_err(meta_item.span,
            "`changeset_for` may only be apllied to enums and structs");
    }
}

fn changeset_tables(cx: &mut ExtCtxt, meta_item: &MetaItem) -> Option<InternedString> {
    match meta_item.node {
        ast::MetaList(_, ref meta_items) => {
            meta_items.iter().filter_map(|i| table_name(cx, i)).nth(0)
        }
        _ => usage_error(cx, meta_item),
    }
}

fn table_name(cx: &mut ExtCtxt, meta_item: &MetaItem) -> Option<InternedString> {
    match meta_item.node {
        ast::MetaWord(ref word) => Some(word.clone()),
        _ => usage_error(cx, meta_item),
    }
}

fn usage_error<T>(cx: &mut ExtCtxt, meta_item: &MetaItem) -> Option<T> {
    cx.span_err(meta_item.span,
        "`changeset_for` must be used in the form `#[changeset_for(table1)]`");
    None
}

fn changeset_impl(
    cx: &mut ExtCtxt,
    builder: aster::AstBuilder,
    table: &str,
    model: &Model,
) -> Option<P<ast::Item>> {
    let ref struct_name = model.ty;
    let pk = model.primary_key_name();
    let attrs_for_changeset = model.attrs.iter().filter(|a| a.column_name != pk)
        .collect::<Vec<_>>();
    let changeset_ty = builder.ty().tuple()
        .with_tys(attrs_for_changeset.iter().map(|a| changeset_ty(cx, builder, table, a)))
        .build();
    let changeset_body = builder.expr().tuple()
        .with_exprs(attrs_for_changeset.iter().map(|a| changeset_expr(cx, builder, table, a)))
        .build();
    quote_item!(cx,
        impl<'a: 'update, 'update> ::yaqb::query_builder::AsChangeset for
            &'update $struct_name
        {
            type Changeset = $changeset_ty;

            fn as_changeset(self) -> Self::Changeset {
                $changeset_body
            }
        }
    )
}

fn save_changes_impl(
    cx: &mut ExtCtxt,
    builder: aster::AstBuilder,
    table: &str,
    model: &Model,
) -> Option<P<ast::Item>> {
    let ref struct_name = model.ty;
    let pk = model.primary_key_name();
    let table = builder.path()
        .segment(table).build()
        .segment("table").build()
        .build();
    model.attrs.iter().find(|a| a.column_name == pk).and_then(|pk| {
        let pk_field = pk.field_name.unwrap();
        quote_item!(cx,
            impl<'a> $struct_name {
                pub fn save_changes(&mut self, connection: &::yaqb::Connection) -> ::yaqb::QueryResult<()> {
                    use ::yaqb::query_builder::update;
                    *self = {
                        let command = update($table.filter($table.primary_key().eq(&self.$pk_field)))
                            .set(&*self);
                        try!(connection.query_one(command)).unwrap()
                    };
                    Ok(())
                }
            }
        )
    })
}

fn changeset_ty(
    cx: &mut ExtCtxt,
    builder: aster::AstBuilder,
    table: &str,
    attr: &Attr,
) -> P<ast::Ty> {
    let column = builder.path()
        .segment(table).build()
        .segment(attr.column_name).build()
        .build();
    let field_ty = &attr.ty;
    quote_ty!(cx,
        ::yaqb::expression::predicates::Eq<
            $column,
            ::yaqb::expression::bound::Bound<
                <$column as ::yaqb::expression::Expression>::SqlType,
                &'update $field_ty,
            >,
        >
    )
}

fn changeset_expr(
    cx: &mut ExtCtxt,
    builder: aster::AstBuilder,
    table: &str,
    attr: &Attr,
) -> P<ast::Expr> {
    let column = builder.path()
        .segment(table).build()
        .segment(attr.column_name).build()
        .build();
    let field_name = &attr.field_name.unwrap();
    quote_expr!(cx, $column.eq(&self.$field_name))
}
