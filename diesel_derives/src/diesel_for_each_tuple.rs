use proc_macro2::{Ident, Span, TokenStream};

#[cfg(not(feature = "32-column-tables"))]
const MAX_TUPLE_SIZE: i32 = 16;
#[cfg(all(not(feature = "64-column-tables"), feature = "32-column-tables"))]
const MAX_TUPLE_SIZE: i32 = 32;
#[cfg(all(not(feature = "128-column-tables"), feature = "64-column-tables"))]
const MAX_TUPLE_SIZE: i32 = 64;
#[cfg(feature = "128-column-tables")]
const MAX_TUPLE_SIZE: i32 = 128;

pub(crate) fn expand(input: Ident) -> TokenStream {
    let call_side = Span::mixed_site();

    let pairs = (0..MAX_TUPLE_SIZE as usize)
        .map(|i| {
            let t = Ident::new(&format!("T{}", i), call_side);
            let st = Ident::new(&format!("ST{}", i), call_side);
            let tt = Ident::new(&format!("TT{}", i), call_side);
            let i = syn::Index::from(i as usize);
            quote!((#i) -> #t, #st, #tt,)
        })
        .collect::<Vec<_>>();

    let mut out = Vec::with_capacity(MAX_TUPLE_SIZE as usize);

    for i in 0..MAX_TUPLE_SIZE {
        let items = &pairs[0..=i as usize];
        let tuple = i + 1;
        out.push(quote! {
            #tuple {
                #(#items)*
            }
        });
    }

    quote! {
        #input! {
            #(#out)*
        }
    }
}
