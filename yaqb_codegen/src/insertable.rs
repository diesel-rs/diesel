use queriable::struct_fields;
use aster;
use syntax::ast::{
    self,
    Item,
    MetaItem,
    MetaItem_,
    Lit_,
};
use syntax::attr;
use syntax::codemap::{Span, Spanned};
use syntax::ext::base::{Annotatable, ExtCtxt};
use syntax::ext::build::AstBuilder;
use syntax::ptr::P;
use syntax::parse::token::{InternedString, str_to_ident};

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
        for body in tables.into_iter().map(|t| insertable_impl(cx, &builder, t, item)) {
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
) -> P<ast::Item> {
    let generics = match item.node {
        ast::ItemStruct(_, ref generics) => generics,
        _ => cx.bug("Expected a struct"),
    };
    let ty = builder.ty().path()
        .segment(item.ident).with_generics(generics.clone()).build().build();
    let table_mod = builder.id(&*table);
    let fields = struct_fields(cx, item);
    let columns_ty = columns_ty(&builder, &table_mod, fields);
    let values_ty = values_ty(cx, &builder, &table_mod, fields);
    let columns_expr = columns_expr(&builder, &table_mod, fields);
    let values_expr = values_expr(cx, &builder, &table_mod, fields);

    quote_item!(cx,
        impl<'a: 'insert, 'insert> ::yaqb::persistable::Insertable<$table_mod::table> for
            &'insert $ty
        {
            type Columns = $columns_ty;

            type Values = ::yaqb::expression::grouped::Grouped<$values_ty>;

            fn columns() -> Self::Columns {
                $columns_expr
            }

            fn values(self) -> Self::Values {
                use ::yaqb::expression::{AsExpression, Expression};
                use ::yaqb::expression::grouped::Grouped;
                Grouped($values_expr)
            }
        }
    ).unwrap()
}

fn tuple_ty_from<F: Fn(&ast::StructField) -> P<ast::Ty>>(
    builder: &aster::AstBuilder,
    fields: &[ast::StructField],
    f: F,
) -> P<ast::Ty> {
    let tys: Vec<_> = fields.iter().map(f).collect();
    if tys.len() == 1 {
        tys[0].clone()
    } else {
        builder.ty().tuple()
            .with_tys(tys)
            .build()
    }
}

fn tuple_expr_from<F: Fn((usize, &ast::StructField)) -> P<ast::Expr>>(
    builder: &aster::AstBuilder,
    fields: &[ast::StructField],
    f: F,
) -> P<ast::Expr> {
    let exprs: Vec<_> = fields.iter().enumerate().map(f).collect();
    if exprs.len() == 1 {
        exprs[0].clone()
    } else {
        builder.expr().tuple()
            .with_exprs(exprs)
            .build()
    }
}

fn columns_ty(
    builder: &aster::AstBuilder,
    table_mod: &ast::Ident,
    fields: &[ast::StructField],
) -> P<ast::Ty> {
    tuple_ty_from(builder, fields,
                  |f| builder.ty().build_path(column_field_ty(builder, table_mod, f)))
}

fn values_ty(
    cx: &ExtCtxt,
    builder: &aster::AstBuilder,
    table_mod: &ast::Ident,
    fields: &[ast::StructField],
) -> P<ast::Ty> {
    tuple_ty_from(builder, fields, |f| {
        let ref field_ty = f.node.ty;
        let column_field_ty = column_field_ty(builder, table_mod, f);
        quote_ty!(cx,
            ::yaqb::expression::helper_types::AsExpr<&'insert $field_ty, $column_field_ty>)
    })
}

fn column_name(field: &ast::StructField) -> ast::Ident {
    field.node.attrs.iter()
        .filter_map(|attr| {
            match attr.node.value.node {
                MetaItem_::MetaNameValue(ref name, Spanned {
                    node: Lit_::LitStr(ref value, _), ..
                }) if name == &"column_name" => {
                    attr::mark_used(&attr);
                    Some(str_to_ident(&value))
                }
                _ => None,
            }
        }).nth(0)
        .or_else(|| field.node.ident())
        .unwrap()
}

fn column_field_ty(
    builder: &aster::AstBuilder,
    table_mod: &ast::Ident,
    field: &ast::StructField,
) -> ast::Path {
    builder.path()
        .segment(table_mod).build()
        .segment(column_name(field)).build()
        .build()
}

fn columns_expr(
    builder: &aster::AstBuilder,
    table_mod: &ast::Ident,
    fields: &[ast::StructField],
) -> P<ast::Expr> {
    tuple_expr_from(builder, fields, |(_, f)|
        builder.expr().build_path(column_field_ty(builder, table_mod, f)))
}

fn values_expr(
    cx: &ExtCtxt,
    builder: &aster::AstBuilder,
    table_mod: &ast::Ident,
    fields: &[ast::StructField],
) -> P<ast::Expr> {
    tuple_expr_from(builder, fields, |(i, f)| {
        let self_ = builder.expr().self_();
        let field_access = match f.node.ident() {
            Some(i) => builder.expr().field(i).build(self_),
            None => builder.expr().tup_field(i).build(self_),
        };
        let field_ty = column_field_ty(builder, table_mod, f);
        quote_expr!(cx,
            AsExpression::<<$field_ty as Expression>::SqlType>::as_expression(&$field_access))
    })
}
