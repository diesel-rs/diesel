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
use parsers::{MysqlType, SqliteType};

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
                warn!(name, "use `#[diesel(table_name = users)]` format instead");
                Ok(Some(StructAttr::TableName(name.clone(), {
                    let lit_str = parse_eq_and_lit_str(name, input)?;
                    lit_str.parse().unwrap_or_abort()
                })))
            }
            "changeset_options" => {
                warn!(
                    name,
                    "use `#[diesel(treat_none_as_null = true)]` format instead"
                );
                let value = parse_changeset_options(name, input)?;
                Ok(Some(StructAttr::TreatNoneAsNull(value.0, value.1)))
            }
            "sql_type" => {
                warn!(name, "use `#[diesel(sql_type = Text)]` format instead");
                Ok(Some(StructAttr::SqlType(name.clone(), {
                    let lit_str = parse_eq_and_lit_str(name, input)?;
                    lit_str.parse().unwrap_or_abort()
                })))
            }
            "primary_key" => {
                warn!(
                    name,
                    "use `#[diesel(primary_key(id1, id2))]` format instead"
                );
                Ok(Some(StructAttr::PrimaryKey(
                    name.clone(),
                    parse_primary_key(name, input)?,
                )))
            }
            "belongs_to" => {
                warn!(
                    name,
                    "use `#[diesel(belongs_to(User, foreign_key = mykey))]` format instead"
                );
                Ok(Some(StructAttr::BelongsTo(
                    name.clone(),
                    parse_belongs_to(name, input)?,
                )))
            }
            "sqlite_type" => {
                warn!(
                    name,
                    "use `#[diesel(sqlite_type(name = \"TypeName\"))]` format instead"
                );
                Ok(Some(StructAttr::SqliteType(
                    name.clone(),
                    SqliteType {
                        name: parse_eq_and_lit_str(name, input)?,
                    },
                )))
            }
            "mysql_type" => {
                warn!(
                    name,
                    "use `#[diesel(mysql_type(name = \"TypeName\"))]` format instead"
                );
                Ok(Some(StructAttr::MysqlType(
                    name.clone(),
                    MysqlType {
                        name: parse_eq_and_lit_str(name, input)?,
                    },
                )))
            }
            "postgres" => {
                warn!(name, "use `#[diesel(postgres_type(name = \"TypeName\", schema = \"public\"))]` format instead");
                Ok(Some(StructAttr::PostgresType(
                    name.clone(),
                    parse_postgres_type(name, input)?,
                )))
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
                warn!(name, "use `#[diesel(column_name = name)]` format instead");
                Ok(Some(FieldAttr::ColumnName(name.clone(), {
                    let lit_str = parse_eq_and_lit_str(name, input)?;
                    lit_str.parse().unwrap_or_abort()
                })))
            }
            "sql_type" => {
                warn!(name, "use `#[diesel(sql_type = Text)]` format instead");
                Ok(Some(FieldAttr::SqlType(name.clone(), {
                    let lit_str = parse_eq_and_lit_str(name, input)?;
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
