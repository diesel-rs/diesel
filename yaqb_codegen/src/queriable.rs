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

pub fn expand_derive_queriable(
    cx: &mut ExtCtxt,
    span: Span,
    meta_item: &MetaItem,
    annotatable: &Annotatable,
    push: &mut FnMut(Annotatable)
) {
    if let Annotatable::Item(ref item) = *annotatable {
        let builder = aster::AstBuilder::new().span(span);

        let ty = builder.ty().path()
            .segment(item.ident).build()
            .build();

        let fields = struct_fields(
            cx,
            &item,
        );

        let row_type = builder.ty().tuple()
            .with_tys(fields.iter().map(|f| f.node.ty.clone()))
            .build();

        let build_impl = struct_literal_with_fields_assigned_to_row_elements(
            &item,
            &builder,
            fields,
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

pub fn struct_fields<'a>(cx: &ExtCtxt, item: &'a Item) -> &'a [ast::StructField] {
    match item.node {
        ast::ItemStruct(ref variant_data, _) => {
            item_struct_fields(cx, variant_data)
        }
        _ => cx.bug("Expected ItemStruct in #[derive(Queriable)]"),
    }
}

fn item_struct_fields<'a>(
    cx: &ExtCtxt,
    variant_data: &'a ast::VariantData,
) -> &'a [ast::StructField] {
    match *variant_data {
        ast::VariantData::Struct(ref fields, _) => {
            fields
        }
        _ => cx.bug("Tuple structs and unit structs are not supported yet in #[derive(Deserialize)]"),
    }
}

fn struct_literal_with_fields_assigned_to_row_elements(
    item: &Item,
    builder: &aster::AstBuilder,
    fields: &[ast::StructField],
) -> P<ast::Expr> {
    let mut build_impl_builder = builder.expr().struct_path(item.ident);
    for (i, field) in fields.iter().enumerate() {
        build_impl_builder = build_impl_builder
            .field(field.node.ident().unwrap())
            .tup_field(i)
            .id("row");
    }
    build_impl_builder.build()
}

