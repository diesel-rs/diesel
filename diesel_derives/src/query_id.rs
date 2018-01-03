use quote::Tokens;
use syn;

use model::Model;
use util::wrap_item_in_const;

pub fn derive(item: syn::DeriveInput) -> Tokens {
    let model = t!(Model::from_item(&item, "QueryId"));

    let query_id_path: &[_] = &["diesel", "query_builder", "QueryId"];
    let mut generics = syn::aster::from_generics(model.generics.clone())
        .add_ty_param_bound(query_id_path)
        .build();

    for ty_param in &mut generics.ty_params {
        ty_param.default = None;
    }

    let struct_ty = &model.ty;
    let struct_name = &model.name;
    let lifetimes = &generics.lifetimes;

    let query_id_ty_params = generics.ty_params.iter()
        .map(|ty_param| &ty_param.ident)
        .map(|ty_param| quote!(<#ty_param as diesel::query_builder::QueryId>::QueryId))
        .collect::<Vec<_>>();
    let has_static_query_id = generics.ty_params.iter()
        .map(|ty_param| &ty_param.ident)
        .map(|ty_param| quote!(<#ty_param as diesel::query_builder::QueryId>::HAS_STATIC_QUERY_ID))
        .collect::<Vec<_>>();

    wrap_item_in_const(
        model.dummy_const_name("QUERY_ID"),
        quote!(
            #[allow(non_camel_case_types)]
            impl#generics diesel::query_builder::QueryId for #struct_ty {
                type QueryId = #struct_name<#(#lifetimes,)* #(#query_id_ty_params,)*>;

                const HAS_STATIC_QUERY_ID: bool = #(#has_static_query_id &&)* true;
            }
        )
    )
}
