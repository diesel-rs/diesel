use proc_macro2::{self, Span, Ident};
use syn;

use util::*;

pub fn derive(item: syn::DeriveInput) -> Result<proc_macro2::TokenStream, Diagnostic> {
    let mut generics = item.generics.clone();
    generics.params.push(parse_quote!(__QS));

    {
        let where_clause = generics.make_where_clause();
        for type_param in item.generics.type_params() {
            where_clause.predicates.push(parse_quote!(#type_param: AppearsOnTable<__QS>));
        }
        where_clause.predicates.push(parse_quote!(Self: Expression));
    }

    let struct_name = &item.ident;
    let (_, ty_generics, _) = item.generics.split_for_impl();
    let (impl_generics, _, where_clause) = generics.split_for_impl();

    let dummy_name = format!("_impl_appears_on_table_for_{}", item.ident);
    Ok(wrap_in_dummy_mod(
        Ident::new(&dummy_name.to_lowercase(), Span::call_site()),
        quote! {
            use diesel::expression::{AppearsOnTable, Expression};

            impl #impl_generics AppearsOnTable<__QS> for #struct_name #ty_generics
            #where_clause
            {
            }
        },
    ))
}
