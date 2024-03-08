use syn::parse::{Parse, ParseStream, Result};
use syn::punctuated::Punctuated;
use syn::token::Comma;
use syn::{Ident, LitStr};

use crate::util::{parse_eq, unknown_attribute, SQLITE_TYPE_NOTE};

enum Attr {
    Name(LitStr),
}

impl Parse for Attr {
    fn parse(input: ParseStream) -> Result<Self> {
        let name: Ident = input.parse()?;
        let name_str = name.to_string();

        match &*name_str {
            "name" => Ok(Attr::Name(parse_eq(input, SQLITE_TYPE_NOTE)?)),

            _ => Err(unknown_attribute(&name, &["name"])),
        }
    }
}

pub struct SqliteType {
    pub name: LitStr,
}

impl Parse for SqliteType {
    fn parse(input: ParseStream) -> Result<Self> {
        let mut name = None;

        for attr in Punctuated::<Attr, Comma>::parse_terminated(input)? {
            match attr {
                Attr::Name(value) => name = Some(value),
            }
        }

        if let Some(name) = name {
            Ok(SqliteType { name })
        } else {
            Err(syn::Error::new(
                input.span(),
                format!(
                    "expected attribute `name`\n\
                     help: The correct format looks like #[diesel({})]",
                    SQLITE_TYPE_NOTE
                ),
            ))
        }
    }
}
