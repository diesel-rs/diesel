use std::fmt::{Display, Formatter};

use proc_macro2::{Span, TokenStream};
use proc_macro_error::ResultExt;
use quote::ToTokens;
use syn::parse::discouraged::Speculative;
use syn::parse::{Parse, ParseStream, Parser, Result};
use syn::punctuated::Punctuated;
use syn::token::Comma;
use syn::{parenthesized, Attribute, Ident, LitBool, LitStr, Path, Type};

use deprecated::ParseDeprecated;
use parsers::{BelongsTo, MysqlType, PostgresType, SqliteType};
use util::{parse_eq, parse_paren, unknown_attribute};

pub enum FieldAttr {
    Embed(Ident),

    ColumnName(Ident, SqlIdentifier),
    SqlType(Ident, Type),
    SerializeAs(Ident, Type),
    DeserializeAs(Ident, Type),
}

#[derive(Clone)]
pub struct SqlIdentifier {
    field_name: String,
    span: Span,
}

impl SqlIdentifier {
    pub fn span(&self) -> Span {
        self.span
    }
}

impl ToTokens for SqlIdentifier {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        Ident::new(&self.field_name, self.span).to_tokens(tokens)
    }
}

impl Display for SqlIdentifier {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.field_name)
    }
}

impl PartialEq<Ident> for SqlIdentifier {
    fn eq(&self, other: &Ident) -> bool {
        *other == self.field_name
    }
}

impl From<&'_ Ident> for SqlIdentifier {
    fn from(ident: &'_ Ident) -> Self {
        Self {
            span: ident.span(),
            field_name: ident.to_string(),
        }
    }
}

impl Parse for SqlIdentifier {
    fn parse(input: ParseStream) -> Result<Self> {
        let fork = input.fork();

        if let Ok(ident) = fork.parse::<Ident>() {
            input.advance_to(&fork);
            Ok((&ident).into())
        } else {
            let name = input.parse::<LitStr>()?;
            Ok(Self {
                field_name: name.value(),
                span: name.span(),
            })
        }
    }
}

impl Parse for FieldAttr {
    fn parse(input: ParseStream) -> Result<Self> {
        let name: Ident = input.parse()?;
        let name_str = name.to_string();

        match &*name_str {
            "embed" => Ok(FieldAttr::Embed(name)),

            "column_name" => Ok(FieldAttr::ColumnName(name, parse_eq(input)?)),
            "sql_type" => Ok(FieldAttr::SqlType(name, parse_eq(input)?)),
            "serialize_as" => Ok(FieldAttr::SerializeAs(name, parse_eq(input)?)),
            "deserialize_as" => Ok(FieldAttr::DeserializeAs(name, parse_eq(input)?)),

            _ => unknown_attribute(
                &name,
                &[
                    "embed",
                    "column_name",
                    "sql_type",
                    "serialize_as",
                    "deserialize_as",
                ],
            ),
        }
    }
}

#[allow(clippy::large_enum_variant)]
pub enum StructAttr {
    Aggregate(Ident),
    NotSized(Ident),
    ForeignDerive(Ident),

    TableName(Ident, Path),
    SqlType(Ident, Type),
    TreatNoneAsDefaultValue(Ident, LitBool),
    TreatNoneAsNull(Ident, LitBool),

    BelongsTo(Ident, BelongsTo),
    MysqlType(Ident, MysqlType),
    SqliteType(Ident, SqliteType),
    PostgresType(Ident, PostgresType),
    PrimaryKey(Ident, Punctuated<Ident, Comma>),
}

impl Parse for StructAttr {
    fn parse(input: ParseStream) -> Result<Self> {
        let name: Ident = input.parse()?;
        let name_str = name.to_string();

        match &*name_str {
            "aggregate" => Ok(StructAttr::Aggregate(name)),
            "not_sized" => Ok(StructAttr::NotSized(name)),
            "foreign_derive" => Ok(StructAttr::ForeignDerive(name)),

            "table_name" => Ok(StructAttr::TableName(name, parse_eq(input)?)),
            "sql_type" => Ok(StructAttr::SqlType(name, parse_eq(input)?)),
            "treat_none_as_default_value" => {
                Ok(StructAttr::TreatNoneAsDefaultValue(name, parse_eq(input)?))
            }
            "treat_none_as_null" => Ok(StructAttr::TreatNoneAsNull(name, parse_eq(input)?)),

            "belongs_to" => Ok(StructAttr::BelongsTo(name, parse_paren(input)?)),
            "mysql_type" => Ok(StructAttr::MysqlType(name, parse_paren(input)?)),
            "sqlite_type" => Ok(StructAttr::SqliteType(name, parse_paren(input)?)),
            "postgres_type" => Ok(StructAttr::PostgresType(name, parse_paren(input)?)),
            "primary_key" => Ok(StructAttr::PrimaryKey(name, {
                let content;
                parenthesized!(content in input);
                content.parse_terminated(Ident::parse)?
            })),

            _ => unknown_attribute(
                &name,
                &[
                    "aggregate",
                    "not_sized",
                    "foreign_derive",
                    "table_name",
                    "sql_type",
                    "treat_none_as_default_value",
                    "treat_none_as_null",
                    "belongs_to",
                    "mysql_type",
                    "sqlite_type",
                    "postgres_type",
                    "primary_key",
                ],
            ),
        }
    }
}

pub fn parse_attributes<T: Parse + ParseDeprecated>(attrs: &[Attribute]) -> Vec<T> {
    attrs
        .iter()
        .flat_map(|attr| {
            if attr.path.is_ident("diesel") {
                attr.parse_args_with(Punctuated::<T, Comma>::parse_terminated)
                    .unwrap_or_abort()
            } else {
                let mut p = Punctuated::new();
                let Attribute { path, tokens, .. } = attr;
                let ident = path.get_ident().map(|f| f.to_string());

                if let "sql_type" | "column_name" | "table_name" | "changeset_options"
                | "primary_key" | "belongs_to" | "sqlite_type" | "mysql_type" | "postgres" =
                    ident.as_deref().unwrap_or_default()
                {
                    let ts = quote!(#path #tokens).into();
                    let value = Parser::parse(T::parse_deprecated, ts).unwrap_or_abort();

                    if let Some(value) = value {
                        p.push_value(value);
                    }
                }

                p
            }
        })
        .collect()
}
