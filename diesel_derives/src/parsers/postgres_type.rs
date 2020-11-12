use proc_macro_error::abort;
use syn::parse::{Parse, ParseStream, Result};
use syn::punctuated::Punctuated;
use syn::token::Comma;
use syn::{Ident, LitInt, LitStr};

use util::{parse_eq, unknown_attribute};

enum Attr {
    Oid(Ident, LitInt),
    ArrayOid(Ident, LitInt),
    Name(Ident, LitStr),
    Schema(Ident, LitStr),
}

impl Parse for Attr {
    fn parse(input: ParseStream) -> Result<Self> {
        let name: Ident = input.parse()?;
        let name_str = name.to_string();

        match &*name_str {
            "oid" => Ok(Attr::Oid(name, parse_eq(input)?)),
            "array_oid" => Ok(Attr::ArrayOid(name, parse_eq(input)?)),
            "name" => Ok(Attr::Name(name, parse_eq(input)?)),
            "schema" => Ok(Attr::Schema(name, parse_eq(input)?)),

            _ => unknown_attribute(&name),
        }
    }
}

pub enum PostgresType {
    Fixed(LitInt, LitInt),
    Lookup(LitStr, Option<LitStr>),
}

impl Parse for PostgresType {
    fn parse(input: ParseStream) -> Result<Self> {
        let mut oid = None;
        let mut array_oid = None;
        let mut name = None;
        let mut schema = None;

        for attr in Punctuated::<Attr, Comma>::parse_terminated(input)? {
            match attr {
                Attr::Oid(_, value) => oid = Some(value),
                Attr::ArrayOid(_, value) => array_oid = Some(value),
                Attr::Name(_, value) => name = Some(value),
                Attr::Schema(_, value) => schema = Some(value),
            }
        }

        if let Some(name) = name {
            if oid.is_some() {
                abort!(oid, "unexpected `oid` when `name` is present");
            } else if array_oid.is_some() {
                abort!(array_oid, "unexpected `array_oid` when `name` is present");
            }

            Ok(PostgresType::Lookup(name, schema))
        } else if let Some(schema) = schema {
            abort!(
                schema, "expected `name` to be also present";
                help = "make sure `name` is present, `#[diesel(postgres_type(name = \"...\", schema = \"{}\"))]`", schema.value()
            );
        } else if let (Some(oid), Some(array_oid)) = (oid, array_oid) {
            Ok(PostgresType::Fixed(oid, array_oid))
        } else {
            abort!(
                input.span(),
                "expected `oid` and `array_oid` attribute or `name` attribute"
            );
        }
    }
}
