use syn::parse::{ParseStream, Result};
use syn::token::Eq;
use syn::{Ident, LitStr};

pub fn parse_eq_and_lit_str(name: Ident, input: ParseStream, help: &str) -> Result<LitStr> {
    if input.is_empty() {
        return Err(syn::Error::new(
            name.span(),
            format!(
                "unexpected end of input, expected `=`\n\
                     help: The correct format looks like `#[diesel({help})]`"
            ),
        ));
    }

    input.parse::<Eq>()?;
    input.parse::<LitStr>()
}
