use diesel_table_macro_syntax::ColumnDef;

use super::{table_data::TableName, TableData, ViewData};

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
    pub record: Option<Vec<ColumnType>>,
    pub max_length: Option<u64>,
}

impl ColumnType {
    pub(crate) fn for_column_def(c: &ColumnDef) -> Result<Self, crate::errors::Error> {
        Self::for_type_path(
            &c.tpe,
            c.max_length
                .as_ref()
                .map(|l| {
                    l.base10_parse::<u64>()
                        .map_err(crate::errors::Error::ColumnLiteralParseError)
                })
                .transpose()?,
        )
    }

    fn for_type_path(
        t: &syn::TypePath,
        max_length: Option<u64>,
    ) -> Result<Self, crate::errors::Error> {
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
            record: None,
            max_length,
        };
        let is_range = last.ident == "Range";
        let is_multirange = last.ident == "Multirange";
        let is_record = last.ident == "Record";

        let sql_name = if !ret.is_nullable
            && !ret.is_array
            && !ret.is_unsigned
            && !is_range
            && !is_multirange
            && !is_record
        {
            if last.ident == "PgLsn" {
                "pg_lsn".to_string()
            } else {
                last.ident.to_string()
            }
        } else if let syn::PathArguments::AngleBracketed(ref args) = last.arguments {
            let arg = args.args.first().expect("There is at least one argument");
            if let syn::GenericArgument::Type(syn::Type::Tuple(t)) = arg {
                ret.record = Some(
                    t.elems
                        .iter()
                        .map(|t| {
                            if let syn::Type::Path(p) = t {
                                Self::for_type_path(p, None)
                            } else {
                                panic!();
                            }
                        })
                        .collect::<Result<_, _>>()?,
                );
                "record".to_owned()
            } else if let syn::GenericArgument::Type(syn::Type::Path(p)) = arg {
                let s = Self::for_type_path(p, max_length)?;
                if is_range {
                    match s.sql_name.to_uppercase().as_str() {
                        "INT4" | "INTEGER" => "int4range".to_owned(),
                        "INT8" | "BIGINT" => "int8range".into(),
                        "NUMERIC" => "numrange".into(),
                        "TIMESTAMP" | "TIMESTAMP WITHOUT TIME ZONE" => "tsrange".into(),
                        "TIMESTAMPTZ" | "TIMESTAMP WITH TIME ZONE" => "tstzrange".into(),
                        "DATE" => "daterange".into(),
                        s => format!("{s}range"),
                    }
                } else if is_multirange {
                    match s.sql_name.to_uppercase().as_str() {
                        "INT4" | "INTEGER" => "int4multirange".to_owned(),
                        "INT8" | "BIGINT" => "int8multirange".into(),
                        "NUMERIC" => "nummultirange".into(),
                        "TIMESTAMP" | "TIMESTAMP WITHOUT TIME ZONE" => "tsmultirange".into(),
                        "TIMESTAMPTZ" | "TIMESTAMP WITH TIME ZONE" => "tstzmultirange".into(),
                        "DATE" => "datemultirange".into(),
                        s => format!("{s}multirange"),
                    }
                } else {
                    if !ret.is_array {
                        ret.is_nullable |= s.is_nullable;
                        ret.is_array |= s.is_array;
                    }
                    ret.is_unsigned |= s.is_unsigned;
                    ret.record = ret.record.or(s.record);
                    s.sql_name
                }
            } else {
                unreachable!("That shouldn't happen")
            }
        } else {
            unreachable!("That shouldn't happen")
        };
        ret.sql_name = sql_name;
        Ok(ret)
    }
}

use std::{
    fmt::{self, Display},
    str::FromStr,
};

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

#[derive(Debug, Clone)]
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

#[derive(Debug, Eq, PartialEq, Copy, Clone)]
pub enum SupportedColumnStructures {
    View,
    Table,
}

#[derive(Debug)]
pub enum ColumnData {
    View(ViewData),
    Table(TableData),
}

impl ColumnData {
    pub fn table_name(&self) -> &TableName {
        match &self {
            Self::Table(table) => &table.name,
            Self::View(view) => &view.name,
        }
    }

    pub fn columns(&self) -> &Vec<ColumnDefinition> {
        match self {
            Self::Table(table) => &table.column_data,
            Self::View(view) => &view.column_data,
        }
    }

    pub fn comment(&self) -> &Option<String> {
        match self {
            Self::Table(table) => &table.comment,
            Self::View(view) => &view.comment,
        }
    }
}

impl Display for SupportedColumnStructures {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let format = match self {
            Self::Table => "BASE TABLE",
            Self::View => "VIEW",
        };
        write!(f, "{format}")
    }
}

impl FromStr for SupportedColumnStructures {
    type Err = ();
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "BASE TABLE" => Ok(Self::Table),
            "VIEW" => Ok(Self::View),
            _ => unreachable!("This should never happen. Read {s}"),
        }
    }
}

impl SupportedColumnStructures {
    pub fn display_all() -> Vec<String> {
        SupportedColumnStructures::all()
            .into_iter()
            .map(|s| s.to_string())
            .collect()
    }
    pub fn all() -> Vec<SupportedColumnStructures> {
        vec![Self::View, Self::Table]
    }
}
