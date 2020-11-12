use proc_macro_error::{emit_warning, ResultExt};
use syn::parse::{Parse, ParseStream, Result};
use syn::punctuated::Punctuated;
use syn::token::Comma;
use syn::{parenthesized, parse, Attribute, Ident, LitBool, Path, Type};

use parsers::{BelongsTo, MysqlType, PostgresType, SqliteType};
use util::{parse_eq, parse_paren, unknown_attribute};

pub enum FieldAttr {
    Embed(Ident),

    ColumnName(Ident, Ident),
    SqlType(Ident, Type),
    SerializeAs(Ident, Type),
    DeserializeAs(Ident, Type),
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

            _ => unknown_attribute(&name),
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

            _ => unknown_attribute(&name),
        }
    }
}

pub fn parse_attributes<T: Parse>(attrs: &[Attribute]) -> Vec<T> {
    attrs
        .iter()
        .flat_map(|attr| {
            if attr.path.is_ident("diesel") {
                attr.parse_args_with(Punctuated::<T, Comma>::parse_terminated)
                    .unwrap_or_abort()
            } else {
                parse_old_attribute(attr)
            }
        })
        .collect()
}

fn parse_old_attribute<T: Parse>(attr: &Attribute) -> Punctuated<T, Comma> {
    attr.path
        .get_ident()
        .and_then(|ident| match &*ident.to_string() {
            "table_name" | "primary_key" | "column_name" | "sql_type" | "changeset_options" => {
                emit_warning!(ident, "#[{}] attribute form is deprecated", ident);

                let Attribute { path, tokens, .. } = attr;

                Some(parse::<T>(quote! { #path #tokens }.into()).unwrap_or_abort())
            }
            _ => None,
        })
        .map_or(Punctuated::new(), |attr| {
            let mut p = Punctuated::new();
            p.push_value(attr);
            p
        })
}
