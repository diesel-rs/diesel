use proc_macro2::Span;
use proc_macro_error::ResultExt;
use syn::parse::{ParseStream, Result};
use syn::Ident;

mod belongs_to;
mod changeset_options;
mod postgres_type;
mod primary_key;
mod utils;

use attrs::{FieldAttr, StructAttr};
use deprecated::belongs_to::parse_belongs_to;
use deprecated::changeset_options::parse_changeset_options;
use deprecated::postgres_type::parse_postgres_type;
use deprecated::primary_key::parse_primary_key;
use deprecated::utils::parse_eq_and_lit_str;
use parsers::{MysqlType, PostgresType, SqliteType};
use util::{COLUMN_NAME_NOTE, MYSQL_TYPE_NOTE, SQLITE_TYPE_NOTE, SQL_TYPE_NOTE, TABLE_NAME_NOTE};

macro_rules! warn {
    ($ident: expr, $help: expr) => {
        warn(
            $ident.span(),
            &format!("#[{}] attribute form is deprecated", $ident),
            $help,
        );
    };
}

pub trait ParseDeprecated: Sized {
    fn parse_deprecated(input: ParseStream) -> Result<Option<Self>>;
}

impl ParseDeprecated for StructAttr {
    fn parse_deprecated(input: ParseStream) -> Result<Option<Self>> {
        let name: Ident = input.parse()?;
        let name_str = name.to_string();

        match &*name_str {
            "table_name" => {
                let lit_str = parse_eq_and_lit_str(name.clone(), input, TABLE_NAME_NOTE)?;
                warn!(
                    name,
                    &format!("use `#[diesel(table_name = {})]` instead", lit_str.value())
                );
                Ok(Some(StructAttr::TableName(name, {
                    lit_str.parse().unwrap_or_abort()
                })))
            }
            "changeset_options" => {
                let (ident, value) = parse_changeset_options(name.clone(), input)?;
                warn!(
                    name,
                    &format!(
                        "use `#[diesel(treat_none_as_null = {})]` instead",
                        value.value
                    )
                );
                Ok(Some(StructAttr::TreatNoneAsNull(ident, value)))
            }
            "sql_type" => {
                let lit_str = parse_eq_and_lit_str(name.clone(), input, SQL_TYPE_NOTE)?;
                warn!(
                    name,
                    &format!("use `#[diesel(sql_type = {})]` instead", lit_str.value())
                );
                Ok(Some(StructAttr::SqlType(name, {
                    lit_str.parse().unwrap_or_abort()
                })))
            }
            "primary_key" => {
                let keys = parse_primary_key(name.clone(), input)?;
                let hint = keys
                    .iter()
                    .map(|i| i.to_string())
                    .collect::<Vec<_>>()
                    .join(", ");
                warn!(
                    name,
                    &format!("use `#[diesel(primary_key({}))]` instead", hint)
                );
                Ok(Some(StructAttr::PrimaryKey(name, keys)))
            }
            "belongs_to" => {
                let belongs_to = parse_belongs_to(name.clone(), input)?;
                let parent = belongs_to
                    .parent
                    .path
                    .segments
                    .iter()
                    .map(|s| s.ident.to_string())
                    .collect::<Vec<_>>()
                    .join("::");
                if let Some(ref key) = belongs_to.foreign_key {
                    warn!(
                        name,
                        &format!(
                            "use `#[diesel(belongs_to({}, foreign_key = {}))]` instead",
                            parent, key
                        )
                    );
                } else {
                    warn!(
                        name,
                        &format!("use `#[diesel(belongs_to({}))]` instead", parent)
                    );
                }
                Ok(Some(StructAttr::BelongsTo(name, belongs_to)))
            }
            "sqlite_type" => {
                let name_value = parse_eq_and_lit_str(name.clone(), input, SQLITE_TYPE_NOTE)?;
                warn!(
                    name,
                    &format!(
                        "use `#[diesel(sqlite_type(name = \"{}\"))]` instead",
                        name_value.value()
                    )
                );
                Ok(Some(StructAttr::SqliteType(
                    name,
                    SqliteType { name: name_value },
                )))
            }
            "mysql_type" => {
                let name_value = parse_eq_and_lit_str(name.clone(), input, MYSQL_TYPE_NOTE)?;
                warn!(
                    name,
                    &format!(
                        "use `#[diesel(mysql_type(name = \"{}\"))]` instead",
                        name_value.value()
                    )
                );
                Ok(Some(StructAttr::MysqlType(
                    name,
                    MysqlType { name: name_value },
                )))
            }
            "postgres" => {
                let pg_type = parse_postgres_type(name.clone(), input)?;
                let msg = match &pg_type {
                    PostgresType::Fixed(oid, array_oid) => format!(
                        "use `#[diesel(postgres_type(oid = {}, array_oid = {}))]` instead",
                        oid.base10_parse::<u32>()?,
                        array_oid.base10_parse::<u32>()?
                    ),
                    PostgresType::Lookup(name, Some(schema)) => format!(
                        "use `#[diesel(postgres_type(name = \"{}\", schema = \"{}\"))]` instead",
                        name.value(),
                        schema.value()
                    ),
                    PostgresType::Lookup(name, None) => format!(
                        "use `#[diesel(postgres_type(name = \"{}\"))]` instead",
                        name.value(),
                    ),
                };

                warn!(name, &msg);
                Ok(Some(StructAttr::PostgresType(name, pg_type)))
            }
            _ => Ok(None),
        }
    }
}

impl ParseDeprecated for FieldAttr {
    fn parse_deprecated(input: ParseStream) -> Result<Option<Self>> {
        let name: Ident = input.parse()?;
        let name_str = name.to_string();

        match &*name_str {
            "column_name" => {
                let lit_str = parse_eq_and_lit_str(name.clone(), input, COLUMN_NAME_NOTE)?;
                warn!(
                    name,
                    &format!("use `#[diesel(column_name = {})]` instead", lit_str.value())
                );
                Ok(Some(FieldAttr::ColumnName(name, {
                    lit_str.parse().unwrap_or_abort()
                })))
            }
            "sql_type" => {
                let lit_str = parse_eq_and_lit_str(name.clone(), input, SQL_TYPE_NOTE)?;
                warn!(
                    name,
                    &format!("use `#[diesel(sql_type = {})]` instead", lit_str.value())
                );
                Ok(Some(FieldAttr::SqlType(name, {
                    lit_str.parse().unwrap_or_abort()
                })))
            }

            _ => Ok(None),
        }
    }
}

#[cfg(feature = "nightly")]
fn warn(_span: Span, message: &str, help: &str) {
    emit_warning!(_span, message; help = help);
}

#[cfg(not(feature = "nightly"))]
fn warn(_span: Span, message: &str, help: &str) {
    eprintln!("warning: {}\n  = help: {}\n", message, help);
}
