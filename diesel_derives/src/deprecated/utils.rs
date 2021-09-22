use syn::parse::{ParseStream, Result};
use syn::token::Eq;
use syn::{Ident, LitStr};

pub fn parse_eq_and_lit_str(name: Ident, input: ParseStream) -> Result<LitStr> {
    if input.is_empty() {
        abort!(name.span(), "unexpected end of input, expected `=`");
    }

    input.parse::<Eq>()?;
    input.parse::<LitStr>()
}
