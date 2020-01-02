use proc_macro2::*;
use syn;

use util::*;

pub fn derive(mut item: syn::DeriveInput) -> Result<TokenStream, Diagnostic> {
    let type_params = item
        .generics
        .type_params()
        .map(|param| param.ident.clone())
        .collect::<Vec<_>>();
    for type_param in type_params {
        let where_clause = item.generics.make_where_clause();
        where_clause
            .predicates
            .push(parse_quote!(#type_param: NonAggregate));
    }

    let (impl_generics, ty_generics, where_clause) = item.generics.split_for_impl();
    let struct_name = &item.ident;

    Ok(wrap_in_dummy_mod(quote! {
        use diesel::expression::NonAggregate;

        impl #impl_generics NonAggregate for #struct_name #ty_generics
        #where_clause
        {
        }
    }))
}
