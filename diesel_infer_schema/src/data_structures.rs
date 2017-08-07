use diesel::*;
#[cfg(feature="uses_information_schema")]
use diesel::backend::Backend;
#[cfg(feature = "sqlite")]
use diesel::sqlite::Sqlite;
use diesel::types::{HasSqlType, FromSqlRow};

#[cfg(feature="uses_information_schema")]
use super::information_schema::UsesInformationSchema;
use super::table_data::TableData;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ColumnInformation {
    pub column_name: String,
    pub type_name: String,
    pub nullable: bool,
}

pub struct ColumnType {
    pub rust_name: String,
    pub is_array: bool,
    pub is_nullable: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ForeignKeyConstraint {
    pub child_table: TableData,
    pub parent_table: TableData,
    pub foreign_key: String,
}

impl ColumnInformation {
    pub fn new<T, U>(column_name: T, type_name: U, nullable: bool) -> Self where
        T: Into<String>,
        U: Into<String>,
    {
        ColumnInformation {
            column_name: column_name.into(),
            type_name: type_name.into(),
            nullable: nullable,
        }
    }
}

#[cfg(feature="uses_information_schema")]
impl<ST, DB> Queryable<ST, DB> for ColumnInformation where
    DB: Backend + UsesInformationSchema + HasSqlType<ST>,
    (String, String, String): FromSqlRow<ST, DB>,
{
    type Row = (String, String, String);

    fn build(row: Self::Row) -> Self {
        ColumnInformation::new(row.0, row.1, row.2 == "YES")
    }
}

#[cfg(feature = "sqlite")]
impl<ST> Queryable<ST, Sqlite> for ColumnInformation where
    Sqlite: HasSqlType<ST>,
    (i32, String, String, bool, Option<String>, bool): FromSqlRow<ST, Sqlite>,
{
    type Row = (i32, String, String, bool, Option<String>, bool);

    fn build(row: Self::Row) -> Self {
        ColumnInformation::new(row.1, row.2, !row.3)
    }
}
