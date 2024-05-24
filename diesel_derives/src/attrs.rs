use std::fmt::{Display, Formatter};

use proc_macro2::{Span, TokenStream};
use quote::ToTokens;
use syn::parse::discouraged::Speculative;
use syn::parse::{Parse, ParseStream, Result};
use syn::punctuated::Punctuated;
use syn::spanned::Spanned;
use syn::token::Comma;
use syn::{Attribute, Expr, Ident, LitBool, LitStr, Path, Type, TypePath};

use crate::deprecated::ParseDeprecated;
use crate::parsers::{BelongsTo, MysqlType, PostgresType, SqliteType};
use crate::util::{
    parse_eq, parse_paren, unknown_attribute, BELONGS_TO_NOTE, COLUMN_NAME_NOTE,
    DESERIALIZE_AS_NOTE, MYSQL_TYPE_NOTE, POSTGRES_TYPE_NOTE, SELECT_EXPRESSION_NOTE,
    SELECT_EXPRESSION_TYPE_NOTE, SERIALIZE_AS_NOTE, SQLITE_TYPE_NOTE, SQL_TYPE_NOTE,
    TABLE_NAME_NOTE, TREAT_NONE_AS_DEFAULT_VALUE_NOTE, TREAT_NONE_AS_NULL_NOTE,
};

use crate::util::{parse_paren_list, CHECK_FOR_BACKEND_NOTE};

pub trait MySpanned {
    fn span(&self) -> Span;
}

pub struct AttributeSpanWrapper<T> {
    pub item: T,
    pub attribute_span: Span,
    pub ident_span: Span,
}

pub enum FieldAttr {
    Embed(Ident),
    SkipInsertion(Ident),

    ColumnName(Ident, SqlIdentifier),
    SqlType(Ident, TypePath),
    TreatNoneAsDefaultValue(Ident, LitBool),
    TreatNoneAsNull(Ident, LitBool),

    SerializeAs(Ident, TypePath),
    DeserializeAs(Ident, TypePath),
    SelectExpression(Ident, Expr),
    SelectExpressionType(Ident, Type),
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

    pub fn to_ident(&self) -> Result<Ident> {
        match syn::parse_str::<Ident>(&format!("r#{}", self.field_name)) {
            Ok(mut ident) => {
                ident.set_span(self.span);
                Ok(ident)
            }
            Err(_e) if self.field_name.contains(' ') => Err(syn::Error::new(
                self.span(),
                format!(
                    "Expected valid identifier, found `{0}`. \
                 Diesel does not support column names with whitespaces yet",
                    self.field_name
                ),
            )),
            Err(_e) => Err(syn::Error::new(
                self.span(),
                format!(
                    "Expected valid identifier, found `{0}`. \
                 Diesel automatically renames invalid identifiers, \
                 perhaps you meant to write `{0}_`?",
                    self.field_name
                ),
            )),
        }
    }
}

impl ToTokens for SqlIdentifier {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        if self.field_name.starts_with("r#") {
            Ident::new_raw(&self.field_name[2..], self.span).to_tokens(tokens)
        } else {
            Ident::new(&self.field_name, self.span).to_tokens(tokens)
        }
    }
}

impl Display for SqlIdentifier {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let mut start = 0;
        if self.field_name.starts_with("r#") {
            start = 2;
        }
        f.write_str(&self.field_name[start..])
    }
}

impl PartialEq<Ident> for SqlIdentifier {
    fn eq(&self, other: &Ident) -> bool {
        *other == self.field_name
    }
}

impl From<&'_ Ident> for SqlIdentifier {
    fn from(ident: &'_ Ident) -> Self {
        use syn::ext::IdentExt;
        let ident = ident.unraw();
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
            "skip_insertion" => Ok(FieldAttr::SkipInsertion(name)),

            "column_name" => Ok(FieldAttr::ColumnName(
                name,
                parse_eq(input, COLUMN_NAME_NOTE)?,
            )),
            "sql_type" => Ok(FieldAttr::SqlType(name, parse_eq(input, SQL_TYPE_NOTE)?)),
            "treat_none_as_default_value" => Ok(FieldAttr::TreatNoneAsDefaultValue(
                name,
                parse_eq(input, TREAT_NONE_AS_DEFAULT_VALUE_NOTE)?,
            )),
            "treat_none_as_null" => Ok(FieldAttr::TreatNoneAsNull(
                name,
                parse_eq(input, TREAT_NONE_AS_NULL_NOTE)?,
            )),
            "serialize_as" => Ok(FieldAttr::SerializeAs(
                name,
                parse_eq(input, SERIALIZE_AS_NOTE)?,
            )),
            "deserialize_as" => Ok(FieldAttr::DeserializeAs(
                name,
                parse_eq(input, DESERIALIZE_AS_NOTE)?,
            )),
            "select_expression" => Ok(FieldAttr::SelectExpression(
                name,
                parse_eq(input, SELECT_EXPRESSION_NOTE)?,
            )),
            "select_expression_type" => Ok(FieldAttr::SelectExpressionType(
                name,
                parse_eq(input, SELECT_EXPRESSION_TYPE_NOTE)?,
            )),
            _ => Err(unknown_attribute(
                &name,
                &[
                    "embed",
                    "skip_insertion",
                    "column_name",
                    "sql_type",
                    "treat_none_as_default_value",
                    "treat_none_as_null",
                    "serialize_as",
                    "deserialize_as",
                    "select_expression",
                    "select_expression_type",
                ],
            )),
        }
    }
}

impl MySpanned for FieldAttr {
    fn span(&self) -> Span {
        match self {
            FieldAttr::Embed(ident)
            | FieldAttr::SkipInsertion(ident)
            | FieldAttr::ColumnName(ident, _)
            | FieldAttr::SqlType(ident, _)
            | FieldAttr::TreatNoneAsNull(ident, _)
            | FieldAttr::TreatNoneAsDefaultValue(ident, _)
            | FieldAttr::SerializeAs(ident, _)
            | FieldAttr::DeserializeAs(ident, _)
            | FieldAttr::SelectExpression(ident, _)
            | FieldAttr::SelectExpressionType(ident, _) => ident.span(),
        }
    }
}

#[allow(clippy::large_enum_variant)]
pub enum StructAttr {
    Aggregate(Ident),
    NotSized(Ident),
    ForeignDerive(Ident),

    TableName(Ident, Path),
    SqlType(Ident, TypePath),
    TreatNoneAsDefaultValue(Ident, LitBool),
    TreatNoneAsNull(Ident, LitBool),

    BelongsTo(Ident, BelongsTo),
    MysqlType(Ident, MysqlType),
    SqliteType(Ident, SqliteType),
    PostgresType(Ident, PostgresType),
    PrimaryKey(Ident, Punctuated<Ident, Comma>),
    CheckForBackend(Ident, syn::punctuated::Punctuated<TypePath, syn::Token![,]>),
}

impl Parse for StructAttr {
    fn parse(input: ParseStream) -> Result<Self> {
        let name: Ident = input.parse()?;
        let name_str = name.to_string();

        match &*name_str {
            "aggregate" => Ok(StructAttr::Aggregate(name)),
            "not_sized" => Ok(StructAttr::NotSized(name)),
            "foreign_derive" => Ok(StructAttr::ForeignDerive(name)),

            "table_name" => Ok(StructAttr::TableName(
                name,
                parse_eq(input, TABLE_NAME_NOTE)?,
            )),
            "sql_type" => Ok(StructAttr::SqlType(name, parse_eq(input, SQL_TYPE_NOTE)?)),
            "treat_none_as_default_value" => Ok(StructAttr::TreatNoneAsDefaultValue(
                name,
                parse_eq(input, TREAT_NONE_AS_DEFAULT_VALUE_NOTE)?,
            )),
            "treat_none_as_null" => Ok(StructAttr::TreatNoneAsNull(
                name,
                parse_eq(input, TREAT_NONE_AS_NULL_NOTE)?,
            )),

            "belongs_to" => Ok(StructAttr::BelongsTo(
                name,
                parse_paren(input, BELONGS_TO_NOTE)?,
            )),
            "mysql_type" => Ok(StructAttr::MysqlType(
                name,
                parse_paren(input, MYSQL_TYPE_NOTE)?,
            )),
            "sqlite_type" => Ok(StructAttr::SqliteType(
                name,
                parse_paren(input, SQLITE_TYPE_NOTE)?,
            )),
            "postgres_type" => Ok(StructAttr::PostgresType(
                name,
                parse_paren(input, POSTGRES_TYPE_NOTE)?,
            )),
            "primary_key" => Ok(StructAttr::PrimaryKey(
                name,
                parse_paren_list(input, "key1, key2", syn::Token![,])?,
            )),
            "check_for_backend" => Ok(StructAttr::CheckForBackend(
                name,
                parse_paren_list(input, CHECK_FOR_BACKEND_NOTE, syn::Token![,])?,
            )),

            _ => Err(unknown_attribute(
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
                    "check_for_backend",
                ],
            )),
        }
    }
}

impl MySpanned for StructAttr {
    fn span(&self) -> Span {
        match self {
            StructAttr::Aggregate(ident)
            | StructAttr::NotSized(ident)
            | StructAttr::ForeignDerive(ident)
            | StructAttr::TableName(ident, _)
            | StructAttr::SqlType(ident, _)
            | StructAttr::TreatNoneAsDefaultValue(ident, _)
            | StructAttr::TreatNoneAsNull(ident, _)
            | StructAttr::BelongsTo(ident, _)
            | StructAttr::MysqlType(ident, _)
            | StructAttr::SqliteType(ident, _)
            | StructAttr::PostgresType(ident, _)
            | StructAttr::CheckForBackend(ident, _)
            | StructAttr::PrimaryKey(ident, _) => ident.span(),
        }
    }
}

pub fn parse_attributes<T>(attrs: &[Attribute]) -> Result<Vec<AttributeSpanWrapper<T>>>
where
    T: Parse + ParseDeprecated + MySpanned,
{
    let mut out = Vec::new();
    for attr in attrs {
        if attr.meta.path().is_ident("diesel") {
            let map = attr
                .parse_args_with(Punctuated::<T, Comma>::parse_terminated)?
                .into_iter()
                .map(|a| AttributeSpanWrapper {
                    ident_span: a.span(),
                    item: a,
                    attribute_span: attr.meta.span(),
                });
            out.extend(map);
        } else if cfg!(all(
            not(feature = "without-deprecated"),
            feature = "with-deprecated"
        )) {
            let path = attr.meta.path();
            let ident = path.get_ident().map(|f| f.to_string());

            if let "sql_type" | "column_name" | "table_name" | "changeset_options" | "primary_key"
            | "belongs_to" | "sqlite_type" | "mysql_type" | "postgres" =
                ident.as_deref().unwrap_or_default()
            {
                let m = &attr.meta;
                let ts = quote::quote!(#m).into();
                let value = syn::parse::Parser::parse(T::parse_deprecated, ts)?;

                if let Some(value) = value {
                    out.push(AttributeSpanWrapper {
                        ident_span: value.span(),
                        item: value,
                        attribute_span: attr.meta.span(),
                    });
                }
            }
        }
    }
    Ok(out)
}
