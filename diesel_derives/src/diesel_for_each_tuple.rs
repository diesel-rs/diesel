use proc_macro2::{Ident, Span, TokenStream};
use quote::quote;

pub(crate) fn expand(input: ForEachTupleInput) -> TokenStream {
    let call_side = Span::mixed_site();

    let pairs = (0..input.max_size as usize)
        .map(|i| {
            let (t, st, tt) = if i == 0 {
                // special case these, as #[doc(fake_variadic)]
                // uses the first generic parameter as `T_n` and `T_n` looks
                // much better than `T0_n`
                let t = Ident::new("T", call_side);
                let st = Ident::new("ST", call_side);
                let tt = Ident::new("TT", call_side);
                (t, st, tt)
            } else {
                let t = Ident::new(&format!("T{i}"), call_side);
                let st = Ident::new(&format!("ST{i}"), call_side);
                let tt = Ident::new(&format!("TT{i}"), call_side);
                (t, st, tt)
            };
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
    max_size: u16,
}

impl syn::parse::Parse for ForEachTupleInput {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let inner = input.parse()?;
        input.parse::<syn::Token![,]>()?;
        let max_size = if input.peek(syn::Ident) {
            let macro_ident = input.parse::<syn::Ident>()?;
            if macro_ident != "env" {
                return Err(syn::Error::new(
                    macro_ident.span(),
                    "only the `env!` macro is expected here",
                ));
            }
            let _bang = input.parse::<syn::Token![!]>()?;
            let name;
            syn::parenthesized!(name in input);
            let s = name.parse::<syn::LitStr>()?;
            std::env::var(s.value())
                .map_err(|_| {
                    syn::Error::new(
                        s.span(),
                        format!("Expected `{}` to be set as environment variable", s.value()),
                    )
                })?
                .parse::<u16>()
                .map_err(|_| {
                    syn::Error::new(
                        s.span(),
                        format!("Expected `{}` to be a u16 integer value", s.value()),
                    )
                })?
        } else {
            input.parse::<syn::LitInt>()?.base10_parse()?
        };
        Ok(Self { inner, max_size })
    }
}
