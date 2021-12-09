use syn::parse::{ParseStream, Result};
use syn::token::Eq;
use syn::{Ident, LitStr};

pub fn parse_eq_and_lit_str(name: Ident, input: ParseStream, help: &str) -> Result<LitStr> {
    if input.is_empty() {
        abort!(
            name.span(),
            "unexpected end of input, expected `=`";
            help = "The correct format looks like `#[diesel({})]`", help
        );
    }

    input.parse::<Eq>()?;
    input.parse::<LitStr>()
}
