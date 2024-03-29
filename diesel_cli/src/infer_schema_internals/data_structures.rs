use diesel_table_macro_syntax::ColumnDef;

use super::table_data::TableName;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ColumnInformation {
    pub column_name: String,
    pub type_name: String,
    pub type_schema: Option<String>,
    pub nullable: bool,
    pub max_length: Option<u64>,
    pub comment: Option<String>,
}

#[derive(Debug, PartialEq, Clone, Eq)]
pub struct ColumnType {
    pub schema: Option<String>,
    pub rust_name: String,
    pub sql_name: String,
    pub is_array: bool,
    pub is_nullable: bool,
    pub is_unsigned: bool,
    pub max_length: Option<u64>,
}

impl ColumnType {
    pub(crate) fn for_column_def(c: &ColumnDef) -> Result<Self, crate::errors::Error> {
        Ok(Self::for_type_path(
            &c.tpe,
            c.max_length
                .as_ref()
                .map(|l| {
                    l.base10_parse::<u64>()
                        .map_err(crate::errors::Error::ColumnLiteralParseError)
                })
                .transpose()?,
        ))
    }

    fn for_type_path(t: &syn::TypePath, max_length: Option<u64>) -> Self {
        let last = t
            .path
            .segments
            .last()
            .expect("At least one segment in this type-path");

        let mut ret = Self {
            schema: None,
            rust_name: last.ident.to_string(),
            sql_name: String::new(),
            is_array: last.ident == "Array",
            is_nullable: last.ident == "Nullable",
            is_unsigned: last.ident == "Unsigned",
            max_length,
        };

        let sql_name = if !ret.is_nullable && !ret.is_array && !ret.is_unsigned {
            last.ident
                .to_string()
                .split('_')
                .collect::<Vec<_>>()
                .join(" ")
        } else if let syn::PathArguments::AngleBracketed(ref args) = last.arguments {
            let arg = args.args.first().expect("There is at least one argument");
            if let syn::GenericArgument::Type(syn::Type::Path(p)) = arg {
                let s = Self::for_type_path(p, max_length);
                ret.is_nullable |= s.is_nullable;
                ret.is_array |= s.is_array;
                ret.is_unsigned |= s.is_unsigned;
                s.sql_name
            } else {
                unreachable!("That shouldn't happen")
            }
        } else {
            unreachable!("That shouldn't happen")
        };
        ret.sql_name = sql_name;
        ret
    }
}

use std::fmt;

impl fmt::Display for ColumnType {
    fn fmt(&self, out: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        if self.is_nullable {
            write!(out, "Nullable<")?;
        }
        if self.is_array {
            write!(out, "Array<Nullable<")?;
        }
        if self.is_unsigned {
            write!(out, "Unsigned<")?;
        }
        write!(out, "{}", self.rust_name)?;
        if self.is_unsigned {
            write!(out, ">")?;
        }
        if self.is_array {
            write!(out, ">>")?;
        }
        if self.is_nullable {
            write!(out, ">")?;
        }
        Ok(())
    }
}

#[derive(Debug)]
pub struct ColumnDefinition {
    pub sql_name: String,
    pub rust_name: String,
    pub ty: ColumnType,
    pub comment: Option<String>,
}

impl ColumnInformation {
    pub fn new<T, U>(
        column_name: T,
        type_name: U,
        type_schema: Option<String>,
        nullable: bool,
        max_length: Option<u64>,
        comment: Option<String>,
    ) -> Self
    where
        T: Into<String>,
        U: Into<String>,
    {
        ColumnInformation {
            column_name: column_name.into(),
            type_name: type_name.into(),
            type_schema,
            nullable,
            max_length,
            comment,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct ForeignKeyConstraint {
    pub child_table: TableName,
    pub parent_table: TableName,
    pub foreign_key_columns: Vec<String>,
    pub foreign_key_columns_rust: Vec<String>,
    pub primary_key_columns: Vec<String>,
}

impl ForeignKeyConstraint {
    pub fn ordered_tables(&self) -> (&TableName, &TableName) {
        use std::cmp::{max, min};
        (
            min(&self.parent_table, &self.child_table),
            max(&self.parent_table, &self.child_table),
        )
    }
}
