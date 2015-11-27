use aster;
use syntax::ast::{
    self,
    Expr,
    Item,
    MetaItem,
};
use syntax::codemap::Span;
use syntax::ext::base::{Annotatable, ExtCtxt};
use syntax::ext::build::AstBuilder;
use syntax::ptr::P;

use attr::Attr;

pub fn expand_derive_queriable(
    cx: &mut ExtCtxt,
    span: Span,
    meta_item: &MetaItem,
    annotatable: &Annotatable,
    push: &mut FnMut(Annotatable)
) {
    if let Annotatable::Item(ref item) = *annotatable {
        let attrs = match Attr::from_item(cx, item) {
            Some((_, attrs)) => attrs,
            None => {
                cx.span_err(span, "`#[derive(Queriable)]` can only be applied to structs or tuple structs");
                return;
            }
        };
        let builder = aster::AstBuilder::new().span(span);

        let ty = builder.ty().path()
            .segment(item.ident).build()
            .build();

        let row_type = builder.ty().tuple()
            .with_tys(attrs.iter().map(|f| f.ty.clone()))
            .build();

        let build_impl = struct_literal_with_fields_assigned_to_row_elements(
            &item,
            &builder,
            &attrs,
        );

        let impl_item = quote_item!(cx,
            impl<__ST> ::yaqb::Queriable<__ST> for $ty where
                __ST: ::yaqb::types::NativeSqlType,
                $row_type: ::yaqb::types::FromSqlRow<__ST>,
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

fn struct_literal_with_fields_assigned_to_row_elements(
    item: &Item,
    builder: &aster::AstBuilder,
    fields: &[Attr],
) -> P<ast::Expr> {
    let mut build_impl_builder = builder.expr().struct_path(item.ident);
    for (i, field) in fields.iter().enumerate() {
        build_impl_builder = build_impl_builder
            .field(field.field_name.unwrap())
            .tup_field(i)
            .id("row");
    }
    build_impl_builder.build()
}
