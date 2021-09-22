use proc_macro_error::ResultExt;
use syn::parse::{ParseStream, Result};
use syn::{parenthesized, Ident, LitBool};

use deprecated::utils::parse_eq_and_lit_str;

pub fn parse_changeset_options(name: Ident, input: ParseStream) -> Result<(Ident, LitBool)> {
    if input.is_empty() {
        abort!(name.span(), "unexpected end of input, expected parentheses");
    }

    let content;
    parenthesized!(content in input);

    let name: Ident = content.parse()?;
    let name_str = name.to_string();

    if name_str != "treat_none_as_null" {
        abort!(name.span(), "expected `treat_none_as_null`");
    }

    Ok((name.clone(), {
        let lit_str = parse_eq_and_lit_str(name, &content)?;
        lit_str.parse().unwrap_or_abort()
    }))
}
