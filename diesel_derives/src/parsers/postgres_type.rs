use proc_macro_error::abort;
use syn::parse::{Parse, ParseStream, Result};
use syn::punctuated::Punctuated;
use syn::token::Comma;
use syn::{Ident, LitInt, LitStr};

use util::{parse_eq, unknown_attribute, POSTGRES_TYPE_NOTE, POSTGRES_TYPE_NOTE_ID};

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
            "oid" => Ok(Attr::Oid(name, parse_eq(input, POSTGRES_TYPE_NOTE_ID)?)),
            "array_oid" => Ok(Attr::ArrayOid(
                name,
                parse_eq(input, POSTGRES_TYPE_NOTE_ID)?,
            )),
            "name" => Ok(Attr::Name(name, parse_eq(input, POSTGRES_TYPE_NOTE)?)),
            "schema" => Ok(Attr::Schema(name, parse_eq(input, POSTGRES_TYPE_NOTE)?)),

            _ => unknown_attribute(&name, &["oid", "array_oid", "name", "schema"]),
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
                Attr::Oid(ident, value) => oid = Some((ident, value)),
                Attr::ArrayOid(ident, value) => array_oid = Some((ident, value)),
                Attr::Name(ident, value) => name = Some((ident, value)),
                Attr::Schema(ident, value) => schema = Some((ident, value)),
            }
        }

        Self::validate_and_build(input, oid, array_oid, name, schema)
    }
}

impl PostgresType {
    pub fn validate_and_build(
        input: ParseStream,
        oid: Option<(Ident, LitInt)>,
        array_oid: Option<(Ident, LitInt)>,
        name: Option<(Ident, LitStr)>,
        schema: Option<(Ident, LitStr)>,
    ) -> Result<Self> {
        let help = format!(
            "The correct format looks like either `#[diesel({})]` or `#[diesel({})]`",
            POSTGRES_TYPE_NOTE, POSTGRES_TYPE_NOTE_ID
        );

        if let Some((_, name)) = name {
            if let Some((oid, _)) = oid {
                abort!(
                    oid, "unexpected `oid` when `name` is present";
                    help = "{}", help
                );
            } else if let Some((array_oid, _)) = array_oid {
                abort!(
                    array_oid, "unexpected `array_oid` when `name` is present";
                    help = "{}", help
                );
            }

            Ok(PostgresType::Lookup(name, schema.map(|s| s.1)))
        } else if let Some((schema, lit)) = schema {
            abort!(
                schema, "expected `name` to be also present";
                help = "make sure `name` is present, `#[diesel(postgres_type(name = \"...\", schema = \"{}\"))]`", lit.value()
            );
        } else if let (Some((_, oid)), Some((_, array_oid))) = (oid, array_oid) {
            Ok(PostgresType::Fixed(oid, array_oid))
        } else {
            abort!(
                input.span(),
                "expected `oid` and `array_oid` attribute or `name` attribute";
                help = "{}", help
            );
        }
    }
}
