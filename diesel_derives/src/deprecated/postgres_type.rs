use syn::parse::{Parse, ParseStream, Result};
use syn::punctuated::Punctuated;
use syn::token::Comma;
use syn::{parenthesized, Ident, LitInt, LitStr};

use deprecated::utils::parse_eq_and_lit_str;
use parsers::PostgresType;
use util::unknown_attribute;

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
                let lit_str = parse_eq_and_lit_str(name, input)?;
                lit_str.parse()?
            })),
            "array_oid" => Ok(Attr::ArrayOid(name.clone(), {
                let lit_str = parse_eq_and_lit_str(name, input)?;
                lit_str.parse()?
            })),
            "type_name" => Ok(Attr::TypeName(
                name.clone(),
                parse_eq_and_lit_str(name, input)?,
            )),

            _ => unknown_attribute(&name),
        }
    }
}

pub fn parse_postgres_type(name: Ident, input: ParseStream) -> Result<PostgresType> {
    if input.is_empty() {
        abort!(name.span(), "unexpected end of input, expected parentheses");
    }

    let content;
    parenthesized!(content in input);

    let mut oid = None;
    let mut array_oid = None;
    let mut type_name = None;

    for attr in Punctuated::<Attr, Comma>::parse_terminated(&content)? {
        match attr {
            Attr::Oid(_, value) => oid = Some(value),
            Attr::ArrayOid(_, value) => array_oid = Some(value),
            Attr::TypeName(_, value) => type_name = Some(value),
        }
    }

    PostgresType::validate_and_build(&content, oid, array_oid, type_name, None)
}
