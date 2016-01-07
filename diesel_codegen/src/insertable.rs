use aster;
use syntax::ast::{
    self,
    Item,
    MetaItem,
    MetaItem_,
};
use syntax::codemap::Span;
use syntax::ext::base::{Annotatable, ExtCtxt};
use syntax::ext::build::AstBuilder;
use syntax::ptr::P;
use syntax::parse::token::InternedString;

use attr::Attr;

pub fn expand_insert(
    cx: &mut ExtCtxt,
    span: Span,
    meta_item: &MetaItem,
    annotatable: &Annotatable,
    push: &mut FnMut(Annotatable)
) {
    if let Annotatable::Item(ref item) = *annotatable {
        let tables = insertable_tables(cx, meta_item);
        let builder = aster::AstBuilder::new().span(span);
        for body in tables.into_iter().filter_map(|t| insertable_impl(cx, &builder, t, item)) {
            push(Annotatable::Item(body));
        }
    } else {
        cx.span_err(meta_item.span,
                    "`insertable_into` may only be applied to enums and structs");
    };
}

fn insertable_tables(cx: &mut ExtCtxt, meta_item: &MetaItem) -> Vec<InternedString> {
    match meta_item.node {
        MetaItem_::MetaList(_, ref meta_items) => {
            meta_items.iter().map(|i| table_name(cx, i)).collect()
        }
        _ => usage_error(cx, meta_item),
    }
}

fn table_name(cx: &mut ExtCtxt, meta_item: &MetaItem) -> InternedString {
    match meta_item.node {
        MetaItem_::MetaWord(ref word) => word.clone(),
        _ => usage_error(cx, meta_item),
    }
}

fn usage_error(cx: &mut ExtCtxt, meta_item: &MetaItem) -> ! {
    cx.span_err(meta_item.span,
        "`insertable_into` must be used in the form `#[insertable_into(table1, table2)]`");
    panic!()
}

fn insertable_impl(
    cx: &mut ExtCtxt,
    builder: &aster::AstBuilder,
    table: InternedString,
    item: &Item,
) -> Option<P<ast::Item>> {
    let (generics, fields) = match Attr::from_item(cx, item) {
        Some(vals) => vals,
        None => {
            cx.span_err(item.span,
                        "Expected a struct or tuple struct for `#[insertable_into]`");
            return None;
        }
    };
    let ty = builder.ty().path()
        .segment(item.ident).with_generics(generics.clone()).build().build();
    let table_mod = builder.id(&*table);
    let columns_ty = columns_ty(&builder, &table_mod, &fields);
    let values_ty = values_ty(cx, &builder, &table_mod, &fields);
    let columns_expr = columns_expr(&builder, &table_mod, &fields);
    let values_expr = values_expr(cx, &builder, &table_mod, &fields);

    quote_item!(cx,
        impl<'a: 'insert, 'insert> ::diesel::persistable::Insertable<$table_mod::table> for
            &'insert $ty
        {
            type Columns = $columns_ty;

            type Values = ::diesel::expression::grouped::Grouped<$values_ty>;

            fn columns() -> Self::Columns {
                $columns_expr
            }

            fn values(self) -> Self::Values {
                use ::diesel::expression::{AsExpression, Expression};
                use ::diesel::expression::grouped::Grouped;
                Grouped($values_expr)
            }
        }
    )
}

fn columns_ty(
    builder: &aster::AstBuilder,
    table_mod: &ast::Ident,
    fields: &[Attr],
) -> P<ast::Ty> {
    tuple_ty_from(builder, fields,
                  |f| builder.ty().build_path(column_field_ty(builder, table_mod, f)))
}

fn values_ty(
    cx: &ExtCtxt,
    builder: &aster::AstBuilder,
    table_mod: &ast::Ident,
    fields: &[Attr],
) -> P<ast::Ty> {
    tuple_ty_from(builder, fields, |f| {
        let ref field_ty = f.ty;
        let column_field_ty = column_field_ty(builder, table_mod, f);
        quote_ty!(cx,
            ::diesel::expression::helper_types::AsExpr<&'insert $field_ty, $column_field_ty>)
    })
}

fn column_field_ty(
    builder: &aster::AstBuilder,
    table_mod: &ast::Ident,
    field: &Attr,
) -> ast::Path {
    builder.path()
        .segment(table_mod).build()
        .segment(field.column_name).build()
        .build()
}

fn columns_expr(
    builder: &aster::AstBuilder,
    table_mod: &ast::Ident,
    fields: &[Attr],
) -> P<ast::Expr> {
    tuple_expr_from(builder, fields, |(_, f)|
        builder.expr().build_path(column_field_ty(builder, table_mod, f)))
}

fn values_expr(
    cx: &ExtCtxt,
    builder: &aster::AstBuilder,
    table_mod: &ast::Ident,
    fields: &[Attr],
) -> P<ast::Expr> {
    tuple_expr_from(builder, fields, |(i, f)| {
        let self_ = builder.expr().self_();
        let field_access = match f.field_name {
            Some(i) => builder.expr().field(i).build(self_),
            None => builder.expr().tup_field(i).build(self_),
        };
        let field_ty = column_field_ty(builder, table_mod, f);
        quote_expr!(cx,
            AsExpression::<<$field_ty as Expression>::SqlType>::as_expression(&$field_access))
    })
}

fn tuple_ty_from<F: Fn(&Attr) -> P<ast::Ty>>(
    builder: &aster::AstBuilder,
    fields: &[Attr],
    f: F,
) -> P<ast::Ty> {
    builder.ty().tuple()
        .with_tys(fields.iter().map(f))
        .build()
}

fn tuple_expr_from<F: Fn((usize, &Attr)) -> P<ast::Expr>>(
    builder: &aster::AstBuilder,
    fields: &[Attr],
    f: F,
) -> P<ast::Expr> {
    builder.expr().tuple()
        .with_exprs(fields.iter().enumerate().map(f))
        .build()
}
