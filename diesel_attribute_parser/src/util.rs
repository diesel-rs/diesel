use syn::parse::{Parse, ParseStream, Peek, Result};
use syn::token::Eq;
use syn::{Ident, Type, parenthesized};

pub fn unknown_attribute(name: &Ident, valid: &[&str]) -> syn::Error {
    let prefix = if valid.len() == 1 { "" } else { " one of" };

    syn::Error::new(
        name.span(),
        format!(
            "unknown attribute, expected{prefix} `{}`",
            valid.join("`, `")
        ),
    )
}

pub fn parse_eq<T: Parse>(input: ParseStream, help: &str) -> Result<T> {
    if input.is_empty() {
        return Err(syn::Error::new(
            input.span(),
            format!(
                "unexpected end of input, expected `=`\n\
                 help: the correct format looks like `#[diesel({help})]`",
            ),
        ));
    }

    input.parse::<Eq>()?;
    input.parse()
}

/// Specialized version of `parse_eq` for `syn::Type` with a customized error message for readability.
/// This is useful because a great variety of tokens would be valid to parse as a `syn::Type`.
pub fn parse_eq_type(input: ParseStream, help: &str) -> Result<syn::Type> {
    if input.is_empty() {
        return Err(syn::Error::new(
            input.span(),
            format!(
                "unexpected end of input, expected `=`\n\
                 help: the correct format looks like `#[diesel({help})]`",
            ),
        ));
    }

    input.parse::<Eq>()?;
    input
        .parse::<Type>()
        .map_err(|e| syn::Error::new(e.span(), "expected type"))
}

pub fn parse_paren<T: Parse>(input: ParseStream, help: &str) -> Result<T> {
    if input.is_empty() {
        return Err(syn::Error::new(
            input.span(),
            format!(
                "unexpected end of input, expected parentheses\n\
                 help: the correct format looks like `#[diesel({help})]`",
            ),
        ));
    }

    let content;
    parenthesized!(content in input);
    content.parse()
}

pub fn parse_paren_list<T, D>(
    input: ParseStream,
    help: &str,
    sep: D,
) -> Result<syn::punctuated::Punctuated<T, <D as Peek>::Token>>
where
    T: Parse,
    D: Peek,
    D::Token: Parse,
{
    if input.is_empty() {
        return Err(syn::Error::new(
            input.span(),
            format!(
                "unexpected end of input, expected parentheses\n\
                 help: the correct format looks like `#[diesel({help})]`",
            ),
        ));
    }

    let content;
    parenthesized!(content in input);
    content.parse_terminated(T::parse, sep)
}
