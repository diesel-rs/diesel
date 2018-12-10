use proc_macro2;
use proc_macro2::*;
use syn;

use util::*;

pub fn derive(mut item: syn::DeriveInput) -> Result<proc_macro2::TokenStream, Diagnostic> {
    for ty_param in item.generics.type_params_mut() {
        ty_param.bounds.push(parse_quote!(QueryId));
    }
    let (impl_generics, ty_generics, where_clause) = item.generics.split_for_impl();

    let struct_name = &item.ident;
    let lifetimes = item.generics.lifetimes();
    let query_id_ty_params = item
        .generics
        .type_params()
        .map(|ty_param| &ty_param.ident)
        .map(|ty_param| quote!(<#ty_param as QueryId>::QueryId));
    let has_static_query_id = item
        .generics
        .type_params()
        .map(|ty_param| &ty_param.ident)
        .map(|ty_param| quote!(<#ty_param as QueryId>::HAS_STATIC_QUERY_ID));

    let dummy_mod = format!("_impl_query_id_for_{}", item.ident).to_lowercase();
    Ok(wrap_in_dummy_mod(
        Ident::new(&dummy_mod, Span::call_site()),
        quote! {
            use diesel::query_builder::QueryId;

            #[allow(non_camel_case_types)]
            impl #impl_generics QueryId for #struct_name #ty_generics
            #where_clause
            {
                type QueryId = #struct_name<#(#lifetimes,)* #(#query_id_ty_params,)*>;

                const HAS_STATIC_QUERY_ID: bool = #(#has_static_query_id &&)* true;
            }
        },
    ))
}
