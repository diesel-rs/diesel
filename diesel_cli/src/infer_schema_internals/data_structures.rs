#[cfg(feature = "uses_information_schema")]
use diesel::backend::Backend;
use diesel::deserialize::{FromSqlRow, Queryable};
#[cfg(feature = "sqlite")]
use diesel::sqlite::Sqlite;

#[cfg(feature = "uses_information_schema")]
use super::information_schema::UsesInformationSchema;
use super::table_data::TableName;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ColumnInformation {
    pub column_name: String,
    pub type_name: String,
    pub nullable: bool,
}

#[derive(Debug)]
pub struct ColumnType {
    pub rust_name: String,
    pub is_array: bool,
    pub is_nullable: bool,
    pub is_unsigned: bool,
}

use std::fmt;

impl fmt::Display for ColumnType {
    fn fmt(&self, out: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        if self.is_nullable {
            write!(out, "Nullable<")?;
        }
        if self.is_array {
            write!(out, "Array<")?;
        }
        if self.is_unsigned {
            write!(out, "Unsigned<")?;
        }
        write!(out, "{}", self.rust_name)?;
        if self.is_unsigned {
            write!(out, ">")?;
        }
        if self.is_array {
            write!(out, ">")?;
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
    pub ty: ColumnType,
    pub docs: String,
    pub rust_name: Option<String>,
}

impl ColumnInformation {
    pub fn new<T, U>(column_name: T, type_name: U, nullable: bool) -> Self
    where
        T: Into<String>,
        U: Into<String>,
    {
        ColumnInformation {
            column_name: column_name.into(),
            type_name: type_name.into(),
            nullable,
        }
    }
}

#[cfg(feature = "uses_information_schema")]
impl<ST, DB> Queryable<ST, DB> for ColumnInformation
where
    DB: Backend + UsesInformationSchema,
    (String, String, String): FromSqlRow<ST, DB>,
{
    type Row = (String, String, String);

    fn build(row: Self::Row) -> Self {
        ColumnInformation::new(row.0, row.1, row.2 == "YES")
    }
}

#[cfg(feature = "sqlite")]
impl<ST> Queryable<ST, Sqlite> for ColumnInformation
where
    (i32, String, String, bool, Option<String>, bool): FromSqlRow<ST, Sqlite>,
{
    type Row = (i32, String, String, bool, Option<String>, bool);

    fn build(row: Self::Row) -> Self {
        ColumnInformation::new(row.1, row.2, !row.3)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct ForeignKeyConstraint {
    pub child_table: TableName,
    pub parent_table: TableName,
    pub foreign_key: String,
    pub primary_key: String,
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
