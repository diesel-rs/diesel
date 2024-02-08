use syn::parse::{Parse, ParseStream, Result};
use syn::punctuated::Punctuated;
use syn::token::Comma;
use syn::{Ident, TypePath};

use crate::util::{parse_eq, unknown_attribute, BELONGS_TO_NOTE};

enum Attr {
    ForeignKey(Ident),
}

impl Parse for Attr {
    fn parse(input: ParseStream) -> Result<Self> {
        let name: Ident = input.parse()?;
        let name_str = name.to_string();

        match &*name_str {
            "foreign_key" => Ok(Attr::ForeignKey(parse_eq(input, BELONGS_TO_NOTE)?)),

            _ => Err(unknown_attribute(&name, &["foreign_key"])),
        }
    }
}

pub struct BelongsTo {
    pub parent: TypePath,
    pub foreign_key: Option<Ident>,
}

impl Parse for BelongsTo {
    fn parse(input: ParseStream) -> Result<Self> {
        let parent = input.parse()?;

        if !input.is_empty() {
            input.parse::<Comma>()?;
        }

        let mut foreign_key = None;

        for attr in Punctuated::<Attr, Comma>::parse_terminated(input)? {
            match attr {
                Attr::ForeignKey(value) => foreign_key = Some(value),
            }
        }

        Ok(BelongsTo {
            parent,
            foreign_key,
        })
    }
}
