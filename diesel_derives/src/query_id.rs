use proc_macro2::TokenStream;
use quote::quote;
use syn::parse_quote;
use syn::DeriveInput;

use crate::util::wrap_in_dummy_mod;

pub fn derive(mut item: DeriveInput) -> TokenStream {
    for ty_param in item.generics.type_params_mut() {
        ty_param
            .bounds
            .push(parse_quote!(diesel::query_builder::QueryId));
    }
    let (impl_generics, ty_generics, where_clause) = item.generics.split_for_impl();
    let is_window_function = item.attrs.iter().any(|a| {
        if a.path().is_ident("diesel") {
            if let Ok(nested) = a.parse_args_with(
                syn::punctuated::Punctuated::<syn::Meta, syn::Token![,]>::parse_terminated,
            ) {
                nested.iter().any(|n| match n {
                    syn::Meta::NameValue(n) => {
                        n.path.is_ident("diesel_internal_is_window")
                            && matches!(
                                n.value,
                                syn::Expr::Lit(syn::ExprLit {
                                    lit: syn::Lit::Bool(syn::LitBool { value: true, .. }),
                                    ..
                                })
                            )
                    }
                    _ => false,
                })
            } else {
                false
            }
        } else {
            false
        }
    });

    let struct_name = &item.ident;
    let lifetimes = item.generics.lifetimes();

    let ty_params = item
        .generics
        .type_params()
        .map(|ty_param| &ty_param.ident)
        .collect::<Vec<_>>();

    let consts = item
        .generics
        .const_params()
        .map(|const_param| &const_param.ident)
        .collect::<Vec<_>>();

    let query_id_ty_params = ty_params
        .iter()
        .map(|ty_param| quote!(<#ty_param as diesel::query_builder::QueryId>::QueryId));
    let has_static_query_id = ty_params
        .iter()
        .map(|ty_param| quote!(<#ty_param as diesel::query_builder::QueryId>::HAS_STATIC_QUERY_ID));
    let is_window_function_list = ty_params
        .iter()
        .map(|ty_param| quote!(<#ty_param as diesel::query_builder::QueryId>::IS_WINDOW_FUNCTION));
    let is_window_function = if is_window_function {
        quote! { true }
    } else {
        quote! { #(#is_window_function_list ||)* false }
    };

    wrap_in_dummy_mod(quote! {
        #[allow(non_camel_case_types)]
        impl #impl_generics diesel::query_builder::QueryId for #struct_name #ty_generics
        #where_clause
        {
            type QueryId = #struct_name<#(#lifetimes,)* #(#query_id_ty_params,)* #(#consts,)*>;

            const HAS_STATIC_QUERY_ID: bool = #(#has_static_query_id &&)* true;

            const IS_WINDOW_FUNCTION: bool = #is_window_function;
        }
    })
}
