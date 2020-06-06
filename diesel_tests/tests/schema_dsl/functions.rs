use diesel::sql_types;

use super::structures::*;

pub fn create_table<Cols>(name: &str, columns: Cols) -> CreateTable<Cols> {
    CreateTable::new(name, columns)
}

pub fn integer(name: &str) -> Column<sql_types::Integer> {
    Column::new(name, "INTEGER")
}

pub fn string(name: &str) -> Column<sql_types::VarChar> {
    Column::new(name, "VARCHAR(255)")
}

pub fn timestamp(name: &str) -> Column<sql_types::VarChar> {
    Column::new(name, "TIMESTAMP")
}

#[cfg(feature = "postgres")]
pub fn timestamptz(name: &str) -> Column<sql_types::VarChar> {
    Column::new(name, "TIMESTAMPTZ")
}

pub fn time(name: &str) -> Column<sql_types::VarChar> {
    Column::new(name, "TIME")
}

pub fn date(name: &str) -> Column<sql_types::VarChar> {
    Column::new(name, "DATE")
}
