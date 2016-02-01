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
use syntax::parse::token::{InternedString, str_to_ident};

use attr::Attr;
use util::{struct_ty, is_option_ty};

pub fn expand_insert(
    cx: &mut ExtCtxt,
    span: Span,
    meta_item: &MetaItem,
    annotatable: &Annotatable,
    push: &mut FnMut(Annotatable)
) {
    if let Annotatable::Item(ref item) = *annotatable {
        let tables = insertable_tables(cx, meta_item);
        for body in tables.into_iter().filter_map(|t| insertable_impl(cx, span, t, item)) {
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
    span: Span,
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
    let ty = struct_ty(cx, span, item.ident, &generics);
    let table_mod = str_to_ident(&table);
    let values_ty = values_ty(cx, span, table_mod, &fields);
    let values_expr = values_expr(cx, span, table_mod, &fields);

    quote_item!(cx,
        impl<'a: 'insert, 'insert, DB> ::diesel::persistable::Insertable<$table_mod::table, DB>
            for &'insert $ty where
                DB: ::diesel::backend::Backend,
                $values_ty: ::diesel::persistable::InsertValues<DB>,
        {
            type Values = $values_ty;

            fn values(self) -> Self::Values {
                use ::diesel::expression::{AsExpression, Expression};
                $values_expr
            }
        }
    )
}

fn values_ty(
    cx: &ExtCtxt,
    span: Span,
    table_mod: ast::Ident,
    fields: &[Attr],
) -> P<ast::Ty> {
    tuple_ty_from(cx, span, fields, |f| {
        let ref field_ty = f.ty;
        let column_field_ty = column_field_ty(cx, span, table_mod, f);
        quote_ty!(cx, ::diesel::persistable::ColumnInsertValue<
            $column_field_ty,
            ::diesel::expression::bound::Bound<
                <$column_field_ty as ::diesel::expression::Expression>::SqlType,
                &'insert $field_ty,
            >,
        >)
    })
}

fn column_field_ty(
    cx: &ExtCtxt,
    span: Span,
    table_mod: ast::Ident,
    field: &Attr,
) -> ast::Path {
    cx.path(span, vec![table_mod, field.column_name])
}

fn values_expr(
    cx: &ExtCtxt,
    span: Span,
    table_mod: ast::Ident,
    fields: &[Attr],
) -> P<ast::Expr> {
    tuple_expr_from(cx, span, fields, |(i, f)| {
        let self_ = cx.expr_self(span);
        let field_access = match f.field_name {
            Some(i) => cx.expr_field_access(span, self_, i),
            None => cx.expr_tup_field_access(span, self_, i),
        };
        let field_ty = column_field_ty(cx, span, table_mod, f);
        let not_none_expr = quote_expr!(cx,
            ::diesel::persistable::ColumnInsertValue::Expression(
                $field_ty,
                AsExpression::<<$field_ty as Expression>::SqlType>::as_expression(&$field_access),
            )
        );
        if is_option_ty(&f.ty) {
            quote_expr!(cx,
                if $field_access.is_none() {
                    ::diesel::persistable::ColumnInsertValue::Default($field_ty)
                } else {
                    $not_none_expr
                }
            )
        } else {
            not_none_expr
        }
    })

}

fn tuple_ty_from<F: Fn(&Attr) -> P<ast::Ty>>(
    cx: &ExtCtxt,
    span: Span,
    fields: &[Attr],
    f: F,
) -> P<ast::Ty> {
    cx.ty(span, ast::TyTup(fields.iter().map(f).collect()))
}

fn tuple_expr_from<F: Fn((usize, &Attr)) -> P<ast::Expr>>(
    cx: &ExtCtxt,
    span: Span,
    fields: &[Attr],
    f: F,
) -> P<ast::Expr> {
    cx.expr_tuple(span, fields.iter().enumerate().map(f).collect())
}
