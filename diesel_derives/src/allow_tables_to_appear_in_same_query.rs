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

    let mut impls = TokenStream::new();
    let tables: Vec<Path> = input.tables.into_iter().collect();

    for (i, left) in tables.iter().enumerate() {
        for right in tables.iter().skip(i + 1) {
            let left_table = quote!(#left::table);
            let right_table = quote!(#right::table);

            impls.extend(quote! {
                impl diesel::query_source::TableNotEqual<#right_table> for #left_table {}
                impl diesel::query_source::TableNotEqual<#left_table> for #right_table {}
            });
        }
    }

    impls
}
