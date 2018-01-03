use quote::Tokens;
use syn;

use model::Model;
use util::wrap_item_in_const;

pub fn derive_queryable(item: syn::DeriveInput) -> Tokens {
    let model = t!(Model::from_item(&item, "Queryable"));

    let generics = syn::aster::from_generics(model.generics.clone())
        .ty_param_id("__DB")
        .ty_param_id("__ST")
        .build();
    let struct_ty = &model.ty;

    let row_ty = model.attrs.iter().map(|a| &a.ty);
    let row_ty = quote!((#(#row_ty,)*));

    let build_expr = build_expr_for_model(&model);

    wrap_item_in_const(
        model.dummy_const_name("QUERYABLE"),
        quote!(
            impl#generics diesel::Queryable<__ST, __DB> for #struct_ty where
                __DB: diesel::backend::Backend + diesel::types::HasSqlType<__ST>,
                #row_ty: diesel::Queryable<__ST, __DB>,
            {
               type Row = <#row_ty as diesel::Queryable<__ST, __DB>>::Row;

               fn build(row: Self::Row) -> Self {
                   let row: #row_ty = diesel::Queryable::build(row);
                   #build_expr
               }
            }
        ),
    )
}

fn build_expr_for_model(model: &Model) -> Tokens {
    let attr_exprs = model.attrs.iter().map(|attr| {
        let name = attr.field_name();
        let idx = &attr.field_position;
        quote!(#name: row.#idx)
    });

    quote!(Self {
        #(#attr_exprs,)*
    })
}
