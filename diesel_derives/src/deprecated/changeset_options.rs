use syn::parse::{ParseStream, Result};
use syn::{parenthesized, Ident, LitBool};

use crate::deprecated::utils::parse_eq_and_lit_str;
use crate::util::TREAT_NONE_AS_NULL_NOTE;

pub fn parse_changeset_options(name: Ident, input: ParseStream) -> Result<(Ident, LitBool)> {
    if input.is_empty() {
        return Err(syn::Error::new(
            name.span(),
            "unexpected end of input, expected parentheses",
        ));
    }

    let content;
    parenthesized!(content in input);

    let name: Ident = content.parse()?;
    let name_str = name.to_string();

    if name_str != "treat_none_as_null" {
        return Err(syn::Error::new(
            name.span(),
            "expected `treat_none_as_null`",
        ));
    }

    Ok((name.clone(), {
        let lit_str = parse_eq_and_lit_str(name, &content, TREAT_NONE_AS_NULL_NOTE)?;
        lit_str.parse()?
    }))
}
