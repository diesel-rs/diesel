use syn::parse::{Parse, ParseStream, Result};
use syn::punctuated::Punctuated;
use syn::token::Comma;
use syn::{parenthesized, Ident, LitInt, LitStr};

use crate::deprecated::utils::parse_eq_and_lit_str;
use crate::parsers::PostgresType;
use crate::util::{unknown_attribute, POSTGRES_TYPE_NOTE};

enum Attr {
    Oid(Ident, LitInt),
    ArrayOid(Ident, LitInt),
    TypeName(Ident, LitStr),
}

impl Parse for Attr {
    fn parse(input: ParseStream) -> Result<Self> {
        let name: Ident = input.parse()?;
        let name_str = name.to_string();

        match &*name_str {
            "oid" => Ok(Attr::Oid(name.clone(), {
                let lit_str = parse_eq_and_lit_str(name, input, POSTGRES_TYPE_NOTE)?;
                lit_str.parse()?
            })),
            "array_oid" => Ok(Attr::ArrayOid(name.clone(), {
                let lit_str = parse_eq_and_lit_str(name, input, POSTGRES_TYPE_NOTE)?;
                lit_str.parse()?
            })),
            "type_name" => Ok(Attr::TypeName(
                name.clone(),
                parse_eq_and_lit_str(name, input, POSTGRES_TYPE_NOTE)?,
            )),

            _ => Err(unknown_attribute(&name, &["oid", "array_oid", "type_name"])),
        }
    }
}

pub fn parse_postgres_type(name: Ident, input: ParseStream) -> Result<PostgresType> {
    if input.is_empty() {
        return Err(syn::Error::new(
            name.span(),
            format!(
                "unexpected end of input, expected parentheses\n\
                 help: The correct format looks like `#[diesel({})]`",
                POSTGRES_TYPE_NOTE
            ),
        ));
    }

    let content;
    parenthesized!(content in input);

    let mut oid = None;
    let mut array_oid = None;
    let mut type_name = None;

    for attr in Punctuated::<Attr, Comma>::parse_terminated(&content)? {
        match attr {
            Attr::Oid(ident, value) => oid = Some((ident, value)),
            Attr::ArrayOid(ident, value) => array_oid = Some((ident, value)),
            Attr::TypeName(ident, value) => type_name = Some((ident, value)),
        }
    }

    PostgresType::validate_and_build(&content, oid, array_oid, type_name, None)
}
