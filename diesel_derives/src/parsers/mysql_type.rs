use syn::parse::{Parse, ParseStream, Result};
use syn::punctuated::Punctuated;
use syn::token::Comma;
use syn::{Ident, LitStr};

use util::{parse_eq, unknown_attribute};

enum Attr {
    Name(Ident, LitStr),
}

impl Parse for Attr {
    fn parse(input: ParseStream) -> Result<Self> {
        let name: Ident = input.parse()?;
        let name_str = name.to_string();

        match &*name_str {
            "name" => Ok(Attr::Name(name, parse_eq(input)?)),

            _ => unknown_attribute(&name),
        }
    }
}

pub struct MysqlType {
    pub name: LitStr,
}

impl Parse for MysqlType {
    fn parse(input: ParseStream) -> Result<Self> {
        let mut name = None;

        for attr in Punctuated::<Attr, Comma>::parse_terminated(input)? {
            match attr {
                Attr::Name(_, value) => name = Some(value),
            }
        }

        if let Some(name) = name {
            Ok(MysqlType { name })
        } else {
            abort!(input.span(), "expected attribute `name`");
        }
    }
}
