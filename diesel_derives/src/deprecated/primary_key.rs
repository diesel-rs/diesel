use syn::parse::{Parse, ParseStream, Result};
use syn::punctuated::Punctuated;
use syn::token::Comma;
use syn::{parenthesized, Ident};

pub fn parse_primary_key(name: Ident, input: ParseStream) -> Result<Punctuated<Ident, Comma>> {
    if input.is_empty() {
        return Err(syn::Error::new(
            name.span(),
            "unexpected end of input, expected parentheses",
        ));
    }

    let content;
    parenthesized!(content in input);

    content.parse_terminated(Ident::parse, syn::Token![,])
}
