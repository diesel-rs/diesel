#[cfg(feature = "uses_information_schema")]
use diesel::backend::Backend;
use diesel::deserialize::{self, FromStaticSqlRow, Queryable};
#[cfg(feature = "sqlite")]
use diesel::sqlite::Sqlite;

#[cfg(feature = "uses_information_schema")]
use super::information_schema::DefaultSchema;
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

impl From<&syn::TypePath> for ColumnType {
    fn from(t: &syn::TypePath) -> Self {
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
            max_length: todo!(),
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
                let s = Self::from(p);
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
        if let Some(max_length) = self.max_length {
            write!(out, " {{{}}}", max_length)?;
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

#[cfg(feature = "uses_information_schema")]
impl<ST, DB> Queryable<ST, DB> for ColumnInformation
where
    DB: Backend + DefaultSchema,
    (
        String,
        String,
        Option<String>,
        String,
        Option<i32>,
        Option<String>,
    ): FromStaticSqlRow<ST, DB>,
{
    type Row = (
        String,
        String,
        Option<String>,
        String,
        Option<i32>,
        Option<String>,
    );

    fn build(row: Self::Row) -> deserialize::Result<Self> {
        Ok(ColumnInformation::new(
            row.0,
            row.1,
            row.2,
            row.3 == "YES",
            row.4.map(std::convert::TryInto::try_into).transpose()?,
            row.5,
        ))
    }
}

#[cfg(feature = "sqlite")]
impl<ST> Queryable<ST, Sqlite> for ColumnInformation
where
    (i32, String, String, bool, Option<String>, bool, i32): FromStaticSqlRow<ST, Sqlite>,
{
    type Row = (i32, String, String, bool, Option<String>, bool, i32);

    fn build(row: Self::Row) -> deserialize::Result<Self> {
        Ok(ColumnInformation::new(
            row.1, row.2, None, !row.3, None, None,
        ))
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
