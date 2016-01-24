use syntax::ast::{
    self,
    Item,
    MetaItem,
};
use syntax::codemap::Span;
use syntax::ext::base::{Annotatable, ExtCtxt};
use syntax::ext::build::AstBuilder;
use syntax::parse::token::*;
use syntax::ptr::P;

use attr::Attr;
use util::struct_ty;

pub fn expand_derive_queryable(
    cx: &mut ExtCtxt,
    span: Span,
    meta_item: &MetaItem,
    annotatable: &Annotatable,
    push: &mut FnMut(Annotatable)
) {
    if let Annotatable::Item(ref item) = *annotatable {
        let (mut generics, attrs) = match Attr::from_item(cx, item) {
            Some((generics, attrs)) => (generics, attrs),
            None => {
                cx.span_err(span, "`#[derive(Queryable)]` can only be applied to structs or tuple structs");
                return;
            }
        };

        let ty = struct_ty(cx, span, item.ident, &generics);

        let row_type = cx.ty(span, ast::TyTup(attrs.iter().map(|f| f.ty.clone()).collect()));

        let build_impl = struct_literal_with_fields_assigned_to_row_elements(
            span, &item, cx, &attrs);
        let mut params = generics.ty_params.into_vec();
        params.push(ty_param_with_name(cx, span, "__ST"));
        params.push(ty_param_with_name(cx, span, "__DB"));
        generics.ty_params = params.into();

        let impl_item = quote_item!(cx,
            impl$generics ::diesel::Queryable<__ST, __DB> for $ty where
                __DB: ::diesel::backend::Backend + ::diesel::types::HasSqlType<__ST>,
                $row_type: ::diesel::types::FromSqlRow<__ST, __DB>,
            {
                type Row = $row_type;

                fn build(row: Self::Row) -> Self {
                    $build_impl
                }
            }
        ).unwrap();

        push(Annotatable::Item(impl_item));
    } else {
        cx.span_err(meta_item.span,
                    "`derive` may only be applied to enums and structs");
    };
}

fn ty_param_with_name(cx: &mut ExtCtxt, span: Span, name: &str) -> ast::TyParam {
    cx.typaram(span, str_to_ident(name), P::empty(), None)
}

fn struct_literal_with_fields_assigned_to_row_elements(
    span: Span,
    item: &Item,
    cx: &mut ExtCtxt,
    fields: &[Attr],
) -> P<ast::Expr> {
    let tup = cx.expr_ident(span, str_to_ident("row"));
    let fields = fields.iter().enumerate().map(|(i, field)| {
        cx.field_imm(
            span,
            field.field_name.unwrap(),
            cx.expr_tup_field_access(span, tup.clone(), i),
        )
    }).collect();
    cx.expr_struct_ident(span, item.ident, fields)
}
