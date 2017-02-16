use quote::Tokens;
use syn;

use attr::Attr;
use model::Model;
use util::wrap_item_in_const;

pub fn derive_queryable(item: syn::MacroInput) -> Tokens {
    let model = t!(Model::from_item(&item, "Queryable"));

    let generics = syn::aster::from_generics(model.generics.clone())
        .ty_param_id("__DB")
        .ty_param_id("__ST")
        .build();
    let struct_ty = &model.ty;

    let row_ty = model.attrs.as_slice().iter().map(|a| &a.ty);
    let row_ty = quote!((#(#row_ty,)*));

    let build_expr = model.build_expr(quote!());
    let field_names = model.attrs.as_slice().iter().map(Attr::name_for_pattern);
    let row_pat = quote!((#(#field_names,)*));

    let model_name_uppercase = model.name.as_ref().to_uppercase();
    let dummy_const = format!("_IMPL_QUERYABLE_FOR_{}", model_name_uppercase).into();

    wrap_item_in_const(dummy_const, quote!(
        impl#generics diesel::Queryable<__ST, __DB> for #struct_ty where
            __DB: diesel::backend::Backend + diesel::types::HasSqlType<__ST>,
            #row_ty: diesel::types::FromSqlRow<__ST, __DB>,
        {
           type Row = #row_ty;

           fn build(#row_pat: Self::Row) -> Self {
               #build_expr
           }
        }
    ))
}

