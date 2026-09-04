use proc_macro2::TokenStream;
use quote::quote;
use syn::parse::{Parse, ParseStream, Result};
use syn::punctuated::Punctuated;
use syn::{Path, Token};

struct AllowTablesToAppearInSameQuery {
    tables: Punctuated<Path, Token![,]>,
}

impl Parse for AllowTablesToAppearInSameQuery {
    fn parse(input: ParseStream) -> Result<Self> {
        Ok(AllowTablesToAppearInSameQuery {
            tables: input.parse_terminated(Path::parse, Token![,])?,
        })
    }
}

pub fn expand(input: TokenStream) -> TokenStream {
    let input: AllowTablesToAppearInSameQuery = match syn::parse2(input) {
        Ok(input) => input,
        Err(e) => return e.to_compile_error(),
    };

    let tables: Vec<Path> = input.tables.into_iter().collect();
    let amount_of_tables = tables.len();
    let mut left_impls = Vec::with_capacity(amount_of_tables * (amount_of_tables - 1));
    let mut right_impls = Vec::with_capacity(amount_of_tables * (amount_of_tables - 1));

    for (i, left) in tables.iter().enumerate() {
        for right in tables.iter().skip(i + 1) {
            left_impls.push(left);
            right_impls.push(right);

            left_impls.push(right);
            right_impls.push(left);
        }
    }

    quote! {
        #(
            impl ::diesel::query_source::TableNotEqual<#right_impls::table> for #left_impls::table {}
        )*
    }
}
