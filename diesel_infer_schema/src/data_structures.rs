use diesel::*;
#[cfg(feature="uses_information_schema")]
use diesel::backend::Backend;
#[cfg(feature = "sqlite")]
use diesel::sqlite::Sqlite;
use diesel::types::{HasSqlType, FromSqlRow};

#[cfg(feature="uses_information_schema")]
use super::information_schema::UsesInformationSchema;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ColumnInformation {
    pub column_name: String,
    pub type_name: String,
    pub nullable: bool,
}

pub struct ColumnType {
    pub path: Vec<String>,
    pub is_array: bool,
    pub is_nullable: bool,
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
    Hlist!(String, String, String): FromSqlRow<ST, DB>,
{
    type Row = Hlist!(String, String, String);

    fn build(hlist_pat!(col, ty, is_nullable): Self::Row) -> Self {
        ColumnInformation::new(col, ty, is_nullable == "YES")
    }
}

#[cfg(feature = "sqlite")]
impl<ST> Queryable<ST, Sqlite> for ColumnInformation where
    Sqlite: HasSqlType<ST>,
    Hlist!(i32, String, String, bool, Option<String>, bool): FromSqlRow<ST, Sqlite>,
{
    type Row = Hlist!(i32, String, String, bool, Option<String>, bool);

    fn build(hlist_pat!(_, col, ty, not_null, _, _): Self::Row) -> Self {
        ColumnInformation::new(col, ty, !not_null)
    }
}
