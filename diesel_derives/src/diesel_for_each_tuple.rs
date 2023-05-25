use proc_macro2::{Ident, Span, TokenStream};
use quote::quote;

#[cfg(not(feature = "32-column-tables"))]
pub const MAX_TUPLE_SIZE: i32 = 16;
#[cfg(all(not(feature = "64-column-tables"), feature = "32-column-tables"))]
pub const MAX_TUPLE_SIZE: i32 = 32;
#[cfg(all(not(feature = "128-column-tables"), feature = "64-column-tables"))]
pub const MAX_TUPLE_SIZE: i32 = 64;
#[cfg(feature = "128-column-tables")]
pub const MAX_TUPLE_SIZE: i32 = 128;

pub(crate) fn expand(input: ForEachTupleInput) -> TokenStream {
    let call_side = Span::mixed_site();

    let pairs = (0..input.max_size as usize)
        .map(|i| {
            let t = Ident::new(&format!("T{i}"), call_side);
            let st = Ident::new(&format!("ST{i}"), call_side);
            let tt = Ident::new(&format!("TT{i}"), call_side);
            let i = syn::Index::from(i);
            quote!((#i) -> #t, #st, #tt,)
        })
        .collect::<Vec<_>>();

    let mut out = Vec::with_capacity(input.max_size as usize);

    for i in 0..input.max_size {
        let items = &pairs[0..=i as usize];
        let tuple = i + 1;
        out.push(quote! {
            #tuple {
                #(#items)*
            }
        });
    }
    let input = input.inner;

    quote! {
        #input! {
            #(#out)*
        }
    }
}

pub struct ForEachTupleInput {
    inner: Ident,
    max_size: i32,
}

impl syn::parse::Parse for ForEachTupleInput {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let inner = input.parse()?;
        let max_size = if input.peek(syn::Token![,]) {
            let _ = input.parse::<syn::Token![,]>();
            input.parse::<syn::LitInt>()?.base10_parse()?
        } else if input.is_empty() {
            MAX_TUPLE_SIZE
        } else {
            unreachable!("Invalid syntax")
        };
        Ok(Self { inner, max_size })
    }
}
