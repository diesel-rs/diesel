use proc_macro2;
use proc_macro2::*;
use syn;

use util::*;

pub fn derive(item: syn::DeriveInput) -> Result<proc_macro2::TokenStream, Diagnostic> {
    let struct_name = &item.ident;

    let chars = struct_name
        .to_string()
        .chars()
        .map(|c| match c {
            c @ 'a'..='z' | c @ 'A'..='Z' => c.to_string(),
            c @ '0'..='9' | c @ '_' => format!("_{}", c),
            _ => panic!("Unsupported name"),
        })
        .collect::<Vec<_>>();
    let chars = chars
        .iter()
        .map(|c| Ident::new(c, Span::call_site()))
        .map(|c| quote!(diesel::frunk::labelled::chars::#c));

    let dummy_name = format!("_impl_named_query_fragment_for_{}", item.ident);
    Ok(wrap_in_dummy_mod(
        Ident::new(&dummy_name.to_lowercase(), Span::call_site()),
        quote! {
            impl diesel::query_builder::NamedQueryFragment for #struct_name {
                type Name = (#(#chars,)*);
            }
        },
    ))
}
