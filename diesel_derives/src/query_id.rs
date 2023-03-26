use proc_macro2::TokenStream;
use quote::quote;
use syn::parse_quote;
use syn::DeriveInput;

use crate::util::wrap_in_dummy_mod;

pub fn derive(mut item: DeriveInput) -> TokenStream {
    for ty_param in item.generics.type_params_mut() {
        ty_param.bounds.push(parse_quote!(QueryId));
    }
    let (impl_generics, ty_generics, where_clause) = item.generics.split_for_impl();

    let struct_name = &item.ident;
    let lifetimes = item.generics.lifetimes();

    let ty_params = item
        .generics
        .type_params()
        .map(|ty_param| &ty_param.ident)
        .collect::<Vec<_>>();

    let query_id_ty_params = ty_params
        .iter()
        .map(|ty_param| quote!(<#ty_param as QueryId>::QueryId));
    let has_static_query_id = ty_params
        .iter()
        .map(|ty_param| quote!(<#ty_param as QueryId>::HAS_STATIC_QUERY_ID));

    wrap_in_dummy_mod(quote! {
        use diesel::query_builder::QueryId;

        #[allow(non_camel_case_types)]
        impl #impl_generics QueryId for #struct_name #ty_generics
        #where_clause
        {
            type QueryId = #struct_name<#(#lifetimes,)* #(#query_id_ty_params,)*>;

            const HAS_STATIC_QUERY_ID: bool = #(#has_static_query_id &&)* true;
        }
    })
}
